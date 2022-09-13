use assembly::{
    fdb::{
        common::Latin1String,
        common::ValueType,
        core::{self, Field},
        mem::Database,
        sqlite::try_export_db,
        store,
    },
    xml::{
        common::{expect_decl, expect_end},
        database::{
            self, expect_column_or_end_columns, expect_columns, expect_database,
            expect_row_or_end_rows, expect_rows, expect_table,
        },
        quick::Reader,
    },
};
use mapr::Mmap;
use rusqlite::{types::ValueRef, Connection};
use std::{
    collections::HashMap,
    fs::File,
    io::{BufReader, BufWriter},
    process::abort,
    time::Instant,
};

fn main() {}

/// Code adapted from
/// https://github.com/LUDevNet/Assembly/blob/fba2c65155e9043fea02c47f14f3d1d86b3f723a/modules/fdb/examples/fdb-to-sqlite.rs#L21
#[no_mangle]
pub extern "C" fn fdb_to_sqlite() {
    let start = Instant::now();

    let src_file = File::open("input").unwrap();
    let mmap = unsafe { Mmap::map(&src_file).unwrap() };
    let buffer: &[u8] = &mmap;

    println!("Copying data, this may take a few seconds...");

    let db = Database::new(buffer);
    let mut conn = Connection::open("output").unwrap();

    try_export_db(&mut conn, db).unwrap();

    let duration = start.elapsed();
    println!(
        "Finished in {}.{}s",
        duration.as_secs(),
        duration.subsec_millis()
    );
}

/// Code adapted from
/// https://github.com/LUDevNet/Assembly/blob/69a4f8383ed3fcd919a0f1a97538e5b13909dd44/modules/fdb/examples/sqlite-to-fdb.rs#L51
#[no_mangle]
pub extern "C" fn sqlite_to_fdb() {
    let start = Instant::now();

    // sqlite input
    let conn =
        Connection::open_with_flags("input", rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();

    // fdb output
    let dest_file = File::create("output").unwrap();

    let mut dest_out = BufWriter::new(dest_file);
    let mut dest_db = store::Database::new();

    println!("Using direct SQLite -> FDB conversion.");
    println!("Converting database, this may take a few seconds...");

    let tables_query = String::from("select name from sqlite_master where type='table'");
    let mut statement = conn.prepare(&tables_query).unwrap();
    let table_names = statement
        .query_map::<String, _, _>([], |row| row.get(0))
        .unwrap();

    for table_name in table_names {
        let table_name = table_name.unwrap();

        // Query used for getting column info and the actual data
        let select_query = format!("select * from {}", &table_name);

        // Prepare statement
        let mut statement = conn.prepare(&select_query).unwrap();

        // Number of columns destination table should have
        let column_count = statement.column_count();

        // Vector to store target datatypes in
        let mut target_types: Vec<ValueType> = Vec::with_capacity(column_count);

        // Get column types
        for column in &statement.columns() {
            let decl_type = column
                .decl_type()
                .expect("The SQLite database is missing column type information. Try converting using a template (see sqlite-to-fdb --help).");

            let target_type = ValueType::from_sqlite_type(decl_type)
                .expect("The SQLite database contains an unknown column type. Try converting using a template (see sqlite-to-fdb --help).");

            target_types.push(target_type);
        }

        // Find number of unique values in first column of source table
        let unique_key_count = conn
            .query_row::<u32, _, _>(
                &format!(
                    "select count(distinct [{}]) as unique_count from {}",
                    statement.column_name(0).unwrap(),
                    table_name
                ),
                [],
                |row| row.get(0),
            )
            .unwrap();

        // Bucket count should be 0 or a power of two
        let new_bucket_count = if unique_key_count == 0 {
            0
        } else {
            u32::next_power_of_two(unique_key_count)
        };

        // Create destination table
        let mut dest_table = store::Table::new(new_bucket_count as usize);

        // Add columns to destination table
        for (i, column) in statement.columns().iter().enumerate() {
            dest_table.push_column(
                Latin1String::encode(column.name()),
                *target_types.get(i).unwrap(),
            );
        }

        // Execute query
        let mut rows = statement.query([]).unwrap();

        // Iterate over rows
        while let Some(sqlite_row) = rows.next().unwrap() {
            let mut fields: Vec<Field> = Vec::with_capacity(column_count);

            // Iterate over fields
            for (index, ty) in target_types.iter().enumerate() {
                // This unwrap is OK because target_types was constructed from the sqlite declaration
                let value = sqlite_row.get_ref(index).unwrap();
                fields.push(match value {
                    ValueRef::Null => Field::Nothing,
                    _ => match ty {
                        ValueType::Nothing => Field::Nothing,
                        ValueType::Integer => Field::Integer(value.as_i64().unwrap() as i32),
                        ValueType::Float => Field::Float(value.as_f64().unwrap() as f32),
                        ValueType::Text => Field::Text(String::from(value.as_str().unwrap())),
                        ValueType::Boolean => Field::Boolean(value.as_i64().unwrap() != 0),
                        ValueType::BigInt => Field::BigInt(value.as_i64().unwrap()),
                        ValueType::VarChar => Field::VarChar(value.as_str().unwrap().to_owned()),
                    },
                });
            }

            // Determine primary key to use for bucket index
            let pk = match &fields[0] {
                Field::Integer(i) => *i as usize,
                Field::BigInt(i) => *i as usize,
                Field::Text(t) => (sfhash::digest(t.as_bytes())) as usize,
                _ => abort(),
            };

            dest_table.push_row(pk, &fields);
        }

        dest_db.push_table(Latin1String::encode(&table_name), dest_table);
    }

    dest_db.write(&mut dest_out).unwrap();

    let duration = start.elapsed();
    println!(
        "\nFinished in {}.{:#03}s",
        duration.as_secs(),
        duration.subsec_millis()
    );

    println!("Output written to 'output'");
}

/// Code adapted from
/// https://github.com/LUDevNet/Assembly/blob/dd2a7e0e9494cc94f7cc4df814f2631d1af1f306/modules/data/examples/xmldb-to-fdb.rs#L38
#[no_mangle]
pub extern "C" fn xml_to_fdb() {
    let src_file = File::open("input").unwrap();
    let reader = BufReader::new(src_file);

    let dest_file = File::create("output").unwrap();
    let mut dest_out = BufWriter::new(dest_file);

    println!("Copying file, this may take a few seconds...");
    let start = Instant::now();

    let mut dest_db = store::Database::new();

    let mut xml = Reader::from_reader(reader);
    let xml = xml.trim_text(true);

    let mut buf = Vec::new();
    let buf = &mut buf;

    expect_decl(xml, buf).unwrap();
    let db_name = expect_database(xml, buf).unwrap().unwrap();
    println!("Loading database: '{}'", db_name);

    while let Some(table_name) = expect_table(xml, buf).unwrap() {
        println!("table '{}'", table_name);
        let mut dest_table = store::Table::new(128);

        expect_columns(xml, buf).unwrap();

        let mut col_map = HashMap::new();

        while let Some(col) = expect_column_or_end_columns(xml, buf).unwrap() {
            let data_type = match col.r#type {
                database::ValueType::Bit => ValueType::Boolean,
                database::ValueType::Float => ValueType::Float,
                database::ValueType::Real => ValueType::Float,
                database::ValueType::Int => ValueType::Integer,
                database::ValueType::BigInt => ValueType::BigInt,
                database::ValueType::SmallInt => ValueType::Integer,
                database::ValueType::TinyInt => ValueType::Integer,
                database::ValueType::Binary => ValueType::Text,
                database::ValueType::VarBinary => ValueType::Text,
                database::ValueType::Char => ValueType::Text,
                database::ValueType::VarChar => ValueType::Text,
                database::ValueType::NChar => ValueType::Text,
                database::ValueType::NVarChar => ValueType::Text,
                database::ValueType::NText => ValueType::VarChar,
                database::ValueType::Text => ValueType::VarChar,
                database::ValueType::Image => ValueType::VarChar,
                database::ValueType::DateTime => ValueType::BigInt,
                database::ValueType::Xml => ValueType::VarChar,
                database::ValueType::Null => ValueType::Nothing,
                database::ValueType::SmallDateTime => ValueType::Integer,
            };
            if col_map.is_empty() {
                // first col
                if data_type == ValueType::Float {
                    let id_col_name = format!("{}ID", table_name);
                    dest_table.push_column(Latin1String::encode(&id_col_name), ValueType::Integer);
                    col_map.insert(id_col_name, col_map.len());
                }
            }

            dest_table.push_column(Latin1String::encode(&col.name), data_type);
            col_map.insert(col.name, col_map.len());
        }

        expect_rows(xml, buf).unwrap();

        let col_count = dest_table.columns().len();
        let mut auto_inc = 0;
        while let Some(row) = expect_row_or_end_rows(xml, buf, true).unwrap() {
            let mut fields = vec![core::Field::Nothing; col_count];
            let mut pk = None;
            for (key, src_value) in row {
                let col_index = *col_map.get(&key).unwrap();
                let value_type = dest_table.columns().get(col_index).unwrap().value_type();
                let dest_value = match value_type {
                    ValueType::Nothing => core::Field::Nothing,
                    ValueType::Integer => core::Field::Integer(src_value.parse().unwrap()),
                    ValueType::Float => core::Field::Float(src_value.parse().unwrap()),
                    ValueType::Text => core::Field::Text(src_value),
                    ValueType::Boolean => core::Field::Boolean(&src_value != "0"),
                    ValueType::BigInt => core::Field::BigInt(src_value.parse().unwrap()),
                    ValueType::VarChar => core::Field::VarChar(src_value),
                };

                if col_index == 0 {
                    match &dest_value {
                        core::Field::Integer(i) => {
                            pk = Some((*i % 128) as usize);
                        }
                        core::Field::BigInt(i) => {
                            pk = Some((*i % 128) as usize);
                        }
                        core::Field::Text(text) => {
                            let lat1 = Latin1String::encode(text);
                            pk = Some((lat1.hash() % 128) as usize);
                        }
                        core::Field::VarChar(var_char) => {
                            let lat1 = Latin1String::encode(var_char);
                            pk = Some((lat1.hash() % 128) as usize);
                        }
                        _ => panic!("Can't use {:?} as PK", &dest_value),
                    }
                }

                fields[col_index] = dest_value;
            }
            let pk = if let Some(pk) = pk {
                pk
            } else {
                auto_inc += 1;
                fields[0] = core::Field::Integer(auto_inc);
                (auto_inc as usize) % 128
            };
            dest_table.push_row(pk, &fields);
        }

        expect_end(xml, buf, "table").unwrap();
        dest_db.push_table(Latin1String::encode(&table_name), dest_table);
    }

    dest_db.write(&mut dest_out).unwrap();

    let duration = start.elapsed();
    println!(
        "Finished in {}.{}s",
        duration.as_secs(),
        duration.subsec_millis()
    );
}

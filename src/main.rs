use assembly::fdb::{
    common::Latin1String, common::ValueType, core::Field, mem::Database, sqlite::try_export_db,
    store,
};
use mapr::Mmap;
use rusqlite::{types::ValueRef, Connection};
use std::{fs::File, io::BufWriter, process::abort, time::Instant};

fn main() {}

/// Code adapted from
/// https://github.com/LUDevNet/Assembly/blob/fba2c65155e9043fea02c47f14f3d1d86b3f723a/modules/fdb/examples/fdb-to-sqlite.rs#L21
#[no_mangle]
pub extern "C" fn fdb_to_sqlite() {
    let start = Instant::now();

    let src_file = File::open("input.fdb").unwrap();
    let mmap = unsafe { Mmap::map(&src_file).unwrap() };
    let buffer: &[u8] = &mmap;

    println!("Copying data, this may take a few seconds...");

    let db = Database::new(buffer);
    let mut conn = Connection::open("output.sqlite").unwrap();

    try_export_db(&mut conn, db).unwrap();

    let duration = start.elapsed();
    println!(
        "Finished in {}.{}s",
        duration.as_secs(),
        duration.subsec_millis()
    );
}

/// Code adapted from
/// https://github.com/LUDevNet/Assembly/blob/168e9f5652afbd2f88ba55bf66c96bcaabff4380/modules/fdb/examples/sqlite-to-fdb.rs#L27
#[no_mangle]
pub extern "C" fn sqlite_to_fdb() {
    let start = Instant::now();

    // sqlite input
    let conn =
        Connection::open_with_flags("input.sqlite", rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();

    // fdb output
    let dest_file = File::create("output.fdb").unwrap();

    let mut dest_out = BufWriter::new(dest_file);
    let mut dest_db = store::Database::new();

    println!("Using direct SQLite -> FDB conversion.");
    println!("Converting database, this may take a few seconds...");

    let tables_query = String::from("select name from sqlite_master where type='table'");
    let mut statement = conn.prepare(&tables_query).unwrap();
    let table_names = statement
        .query_map::<String, _, _>(rusqlite::NO_PARAMS, |row| row.get(0))
        .unwrap();

    for table_name in table_names {
        let table_name = table_name.unwrap();

        // Query used for getting column info and the actual data
        let select_query = format!("select * from {}", &table_name);

        // Prepare statement
        let mut statement = conn.prepare(&select_query).unwrap();

        // Number of columns destination table should have
        let column_count = statement.columns().len();

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
                    statement.columns().get(0).unwrap().name(),
                    table_name
                ),
                rusqlite::NO_PARAMS,
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
        let mut rows = statement.query(rusqlite::NO_PARAMS).unwrap();

        // Iterate over rows
        while let Some(sqlite_row) = rows.next().unwrap() {
            let mut fields: Vec<Field> = Vec::with_capacity(column_count);

            // Iterate over fields
            for index in 0..column_count {
                let value = sqlite_row.get_raw(index);
                fields.push(match value {
                    ValueRef::Null => Field::Nothing,
                    _ => match target_types[index] {
                        ValueType::Nothing => Field::Nothing,
                        ValueType::Integer => Field::Integer(value.as_i64().unwrap() as i32),
                        ValueType::Float => Field::Float(value.as_f64().unwrap() as f32),
                        ValueType::Text => Field::Text(String::from(value.as_str().unwrap())),
                        ValueType::Boolean => Field::Boolean(value.as_i64().unwrap() != 0),
                        ValueType::BigInt => Field::BigInt(value.as_i64().unwrap()),
                        ValueType::VarChar => {
                            Field::VarChar(String::from(value.as_str().unwrap() as &str))
                        }
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
        "Finished in {}.{:#03}s",
        duration.as_secs(),
        duration.subsec_millis()
    );

    println!("Output written to 'output.fdb'");
}

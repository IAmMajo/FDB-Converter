use assembly::fdb::{mem::Database, sqlite::try_export_db};
use mapr::Mmap;
use rusqlite::Connection;
use std::{fs::File, time::Instant};

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

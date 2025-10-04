use r2d2_sqlite::SqliteConnectionManager;
use r2d2::Pool;

// connection pool
pub fn init_pool(path: &str) -> Pool<SqliteConnectionManager> {
    let manager = SqliteConnectionManager::file(path);
    Pool::new(manager).expect("Failed to create pool")
}

// metadata query
pub fn load_metadata_sqlite(path: &str) -> rusqlite::Result<Vec<(String, String, String)>> {
    let conn = Connection::open(path)?;
    let mut stmt = conn.prepare("
        SELECT DISTINCT method, dataset, metric FROM trace_data
    ")?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?))
    })?;
    Ok(rows.map(|r| r.unwrap()).collect())
}

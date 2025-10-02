use rusqlite::Connection;
use bincode;

conn.execute(
    "CREATE TABLE IF NOT EXISTS plotly_traces (
        dataset TEXT, method TEXT, fnr REAL, xs BLOB, ys BLOB
    )",
    [],
)?;

for t in traces.iter() {
    conn.execute(
        "INSERT INTO plotly_traces (dataset, method, fnr, xs, ys) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            t.dataset,
            t.method,
            t.fnr,
            bincode::serialize(&t.xs).unwrap(),
            bincode::serialize(&t.ys).unwrap(),
        ],
    )?;
}

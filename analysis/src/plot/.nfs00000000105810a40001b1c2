use rusqlite::Connection;
use bincode;

struct TraceData {
    method: String,
    dataset: String,
    drop_level: f64,
    fnr: f64,
    metric: String,            // segmentation quality metric (Dice, ADPL, sDice)
    aggregation: String,
    metric_aggregation: String,
    xs: Vec<f64>,
    ys: Vec<f64>,
    th: Vec<f64>,
}



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

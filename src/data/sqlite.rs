use rusqlite::Connection;
use crate::data::loader::TraceData;
use rusqlite::{params, Connection, NO_PARAMS};

pub fn load_traces_sqlite(path: &str) -> rusqlite::Result<Vec<TraceData>> {
    let conn = Connection::open(path)?;
    let mut stmt = conn.prepare("SELECT method,dataset,drop_level,fnr,metric,aggregation,metric_aggregation,xs,th,ys FROM trace_data")?;
    let trace_iter = stmt.query_map([], |row| {
        Ok(TraceData {
            method: row.get(0)?,
            dataset: row.get(1)?,
            drop_level: row.get(2)?,
            fnr: row.get(3)?,
            metric: row.get(4)?,
            aggregation: row.get(5)?,
            metric_aggregation: row.get(6)?,
            xs: bincode::deserialize(&row.get::<_, Vec<u8>>(7)?).unwrap(),
            th: bincode::deserialize(&row.get::<_, Vec<u8>>(8)?).unwrap(),
            ys: bincode::deserialize(&row.get::<_, Vec<u8>>(9)?).unwrap(),
        })
    })?;

    Ok(trace_iter.map(|t| t.unwrap()).collect())
}

pub fn save_traces_sqlite(traces: &[TraceData], path: &str) -> rusqlite::Result<()> {
    let _ = std::fs::remove_file(path);
    let conn = Connection::open(path)?;
    conn.execute(
        "CREATE TABLE trace_data (
            method TEXT, dataset TEXT, drop_level REAL, fnr REAL,
            metric TEXT, aggregation TEXT, metric_aggregation TEXT,
            xs BLOB, th BLOB, ys BLOB
        )",
        NO_PARAMS,
    )?;

    let mut stmt = conn.prepare(
        "INSERT INTO trace_data (method,dataset,drop_level,fnr,metric,aggregation,metric_aggregation,xs,th,ys)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)"
    )?;

    for t in traces {
        stmt.execute(params![
            t.method,
            t.dataset,
            t.drop_level,
            t.fnr,
            t.metric,
            t.aggregation,
            t.metric_aggregation,
            bincode::serialize(&t.xs).unwrap(),
            bincode::serialize(&t.th).unwrap(),
            bincode::serialize(&t.ys).unwrap(),
        ])?;
    }

    Ok(())
}
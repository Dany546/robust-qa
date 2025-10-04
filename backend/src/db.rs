// backend/src/db.rs

//! Role: Encapsulates all interactions with SQLite
//!
//! Responsibilities:
//! - Connection management (supports WAL mode if needed)
//! - Data access functions
//! - Query optimization: only load needed data for current view

use rusqlite::{params, Connection, Result};
use crate::models::{TraceData, TraceMeta};
use bincode;

/// Initialize SQLite connection with optional WAL mode
pub fn init_connection(path: &str, wal: bool) -> Result<Connection> {
    let conn = Connection::open(path)?;
    if wal {
        conn.pragma_update(None, "journal_mode", &"WAL")?;
    }
    Ok(conn)
}

/// Load all traces (full xs, ys, th arrays)
pub fn load_traces(conn: &Connection) -> Result<Vec<TraceData>> {
    let mut stmt = conn.prepare(
        "SELECT method, dataset, drop_level, fnr, metric, aggregation, metric_aggregation, xs, th, ys
         FROM trace_data"
    )?;

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

/// Load traces matching optional filters
pub fn load_filtered_traces(
    conn: &Connection,
    method: Option<&str>,
    dataset: Option<&str>,
    metric: Option<&str>,
    aggregation: Option<&str>,
    metric_aggregation: Option<&str>,
    drop_level: Option<f64>,
) -> Result<Vec<TraceData>> {
    let mut query = String::from(
        "SELECT method, dataset, drop_level, fnr, metric, aggregation, metric_aggregation, xs, th, ys
         FROM trace_data WHERE 1=1"
    );
    let mut params_vec: Vec<rusqlite::types::Value> = vec![];

    if let Some(m) = method {
        query.push_str(" AND method = ?1");
        params_vec.push(m.into());
    }
    if let Some(d) = dataset {
        query.push_str(" AND dataset = ?");
        params_vec.push(d.into());
    }
    if let Some(met) = metric {
        query.push_str(" AND metric = ?");
        params_vec.push(met.into());
    }
    if let Some(agg) = aggregation {
        query.push_str(" AND aggregation = ?");
        params_vec.push(agg.into());
    }
    if let Some(mag) = metric_aggregation {
        query.push_str(" AND metric_aggregation = ?");
        params_vec.push(mag.into());
    }
    if let Some(dl) = drop_level {
        query.push_str(" AND drop_level = ?");
        params_vec.push(dl.into());
    }

    let mut stmt = conn.prepare(&query)?;
    let trace_iter = stmt.query_map(params_vec.as_slice(), |row| {
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

/// Load only metadata for dropdowns
pub fn load_metadata(conn: &Connection) -> Result<TraceMeta> {
    let methods: Vec<String> = conn.prepare("SELECT DISTINCT method FROM trace_data")?
        .query_map([], |r| r.get(0))?
        .map(|r| r.unwrap())
        .collect();

    let datasets: Vec<String> = conn.prepare("SELECT DISTINCT dataset FROM trace_data")?
        .query_map([], |r| r.get(0))?
        .map(|r| r.unwrap())
        .collect();

    let metrics: Vec<String> = conn.prepare("SELECT DISTINCT metric FROM trace_data")?
        .query_map([], |r| r.get(0))?
        .map(|r| r.unwrap())
        .collect();

    let aggregations: Vec<String> = conn.prepare("SELECT DISTINCT aggregation FROM trace_data")?
        .query_map([], |r| r.get(0))?
        .map(|r| r.unwrap())
        .collect();

    let metric_aggs: Vec<String> = conn.prepare("SELECT DISTINCT metric_aggregation FROM trace_data")?
        .query_map([], |r| r.get(0))?
        .map(|r| r.unwrap())
        .collect();

    let drop_levels: Vec<f64> = conn.prepare("SELECT DISTINCT drop_level FROM trace_data")?
        .query_map([], |r| r.get(0))?
        .map(|r| r.unwrap())
        .collect();

    Ok(TraceMeta {
        methods,
        datasets,
        metrics,
        aggregations,
        metric_aggs,
        drop_levels,
    })
}

/// Save traces (full xs, ys, th) into SQLite
pub fn save_traces(traces: &[TraceData], path: &str) -> Result<()> {
    let _ = std::fs::remove_file(path);
    let conn = Connection::open(path)?;
    conn.execute(
        "CREATE TABLE trace_data (
            method TEXT, dataset TEXT, drop_level REAL, fnr REAL,
            metric TEXT, aggregation TEXT, metric_aggregation TEXT,
            xs BLOB, th BLOB, ys BLOB
        )",
        [],
    )?;

    let mut stmt = conn.prepare(
        "INSERT INTO trace_data (method,dataset,drop_level,fnr,metric,aggregation,metric_aggregation,xs,th,ys)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)"
    )?;

    for t in traces {
        stmt.execute(rusqlite::params![
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

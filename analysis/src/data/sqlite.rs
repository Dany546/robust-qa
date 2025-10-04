use rusqlite::{params, Connection, ToSql};
use rayon::prelude::*; 
use std::collections::{HashMap}; 
use crate::data::loader::TraceData;
use crate::data::utils::{make_xy_column_names}; 
use crate::data::columnar::ColumnarTable;
use tokio::task;
use tokio::sync::mpsc;
use serde_json::Value;
use std::sync::mpsc::Receiver;

#[derive(Clone)]
pub struct TraceRow {
    pub x_col: String,
    pub y_col: String,
    pub fnr: f64,
    pub drop_level: f64,
    pub th: Vec<f64>,
    pub xs: Vec<f64>,
    pub ys: Vec<f64>,
}


pub fn save_traces(
    rx: Receiver<TraceRow>,
    db_path: String,
) -> anyhow::Result<()> {
    let mut conn = Connection::open(db_path)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS traces (
            x_col TEXT, y_col TEXT, fnr REAL, drop_level REAL,
            th BLOB, xs BLOB, ys BLOB
        );",
    )?;

    let tx = conn.transaction()?;
    for row in rx {
        // --- PANIC CHECK ---
        if row.th.is_empty() || row.ys.is_empty() {
            panic!(
                "TraceRow has empty vector! x_col={}, y_col={}, fnr={}, drop_level={} {} {}",
                row.x_col, row.y_col, row.fnr, row.drop_level, row.th.is_empty(), row.ys.is_empty()
            );
        }
        tx.execute(
            "INSERT INTO traces (x_col, y_col, fnr, drop_level, th, xs, ys) VALUES (?1,?2,?3,?4,?5,?6,?7)",
            params![
                row.x_col,
                row.y_col,
                row.fnr,
                row.drop_level,
                bincode::serialize(&row.th)?,
                bincode::serialize(&row.xs)?,
                bincode::serialize(&row.ys)?,
            ],
        )?;
    }
    tx.commit()?;
    Ok(())
}

pub async fn save_traces_sqlite(
    traces_table: &ColumnarTable,
    combos: &[(String, f64, String, String, String)],
    pref: &str,
    path: &str,
) -> rusqlite::Result<()> {
    let traces_table = traces_table.clone();
    let combos = combos.to_vec();
    let pref = pref.to_string();
    let path = path.to_string();

    task::spawn_blocking(move || {
        // Remove old file if exists
        let _ = std::fs::remove_file(&path);

        let mut conn = Connection::open(&path)?;

        // Create table
        conn.execute(
            "CREATE TABLE trace_data (
                method TEXT, dataset TEXT, drop_level REAL, fnr REAL,
                metric TEXT, aggregation TEXT, metric_aggregation TEXT,
                th BLOB, xs BLOB, ys BLOB
            )",
            [],
        )?;

        let tx = conn.transaction()?; // wrap all inserts in a transaction
        let mut stmt = tx.prepare(
            "INSERT INTO trace_data
                (method,dataset,drop_level,fnr,metric,aggregation,metric_aggregation,th,xs,ys)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)"
        )?;

        for (method, drop_level, metric, agg, met_agg) in combos {
            let (x_col, y_col) =
                make_xy_column_names(&method, drop_level, &metric, &agg, &met_agg, &pref);

            let xs = traces_table.column_f64(&x_col);
            let ys = traces_table.column_f64(&y_col);
            let th = traces_table.column_f64("th");
            // ===== DEBUG PRINT =====
            println!(
                "[DEBUG] Columns: x_col={} y_col={} th | Sizes: xs={} ys={} th={}",
                x_col, y_col, xs.len(), ys.len(), th.len()
            );

            if xs.is_empty() || ys.is_empty() || th.is_empty() {
                panic!(
                    "Warning: skipping empty columns: xs={} ys={} th={}",
                    xs.len(), ys.len(), th.len()
                );
            }

            let trace = TraceData {
                method: method.clone(),
                dataset: "".to_string(),
                drop_level,
                fnr: f64::NAN,
                metric: metric.clone(),
                aggregation: agg.clone(),
                metric_aggregation: met_agg.clone(),
                th: th.to_vec(),
                xs: xs.to_vec(),
                ys: ys.to_vec(),
            };
            
            let xs_blob = bincode::serialize(&trace.xs)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
            let th_blob = bincode::serialize(&trace.th)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
            let ys_blob = bincode::serialize(&trace.ys)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;


            stmt.execute(params![
                trace.method,
                trace.dataset,
                trace.drop_level,
                trace.fnr,
                trace.metric,
                trace.aggregation,
                trace.metric_aggregation,
                th_blob,
                ys_blob,
                xs_blob,
            ])?;
        }
        drop(stmt);   // release the borrow on tx
        tx.commit()?;
        Ok(())
    })
    .await
    .expect("Tokio task failed")
}

pub fn load_traces_sqlite(path: &str) -> Result<ColumnarTable, anyhow::Error> {
    let conn = Connection::open(path)?;
    let mut stmt = conn.prepare(
        "SELECT method,dataset,drop_level,fnr,metric,aggregation,metric_aggregation,th,xs,ys FROM trace_data"
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
            th: bincode::deserialize(&row.get::<_, Vec<u8>>(8)?).unwrap_or_default(),
            xs: bincode::deserialize(&row.get::<_, Vec<u8>>(7)?).unwrap_or_default(),
            ys: bincode::deserialize(&row.get::<_, Vec<u8>>(9)?).unwrap_or_default(),
        })
    })?;
    
    let mut ct = ColumnarTable {
        n_rows: 0,
        data: HashMap::new(),
        vector_data: HashMap::new(),
    };
    
    for t_res in trace_iter {
        match t_res {
            Ok(t) => {
                // Build column names consistent with save_traces_sqlite
                let (x_col, y_col) =
                    make_xy_column_names(&t.method, t.drop_level, &t.metric, &t.aggregation, &t.metric_aggregation, "");
                
                ct.data.entry(x_col).or_default().extend(&t.xs);
                ct.data.entry(y_col).or_default().extend(&t.ys);
                ct.data.entry("th".to_string()).or_default().extend(&t.th);
            }
            Err(e) => eprintln!("Failed to read trace row: {:?}", e),
        }
    }
    
    ct.n_rows = ct.data.values().next().map(|v| v.len()).unwrap_or(0);
    Ok(ct)
}


pub async fn save_to_sqlite_parallel(
    sqlite_path: String,
    needed_cols: Vec<String>,
    columns: HashMap<String, Vec<f64>>,
) -> Result<(), anyhow::Error> {
    let sqlite_path = sqlite_path.clone();
    let needed_cols = needed_cols.clone();
    let columns = columns.clone();
    let batch_size = 500;

    task::spawn_blocking(move || -> Result<(), anyhow::Error> {
        let n_rows = columns
            .get(&needed_cols[0])
            .ok_or_else(|| anyhow::anyhow!("Column {} not found", needed_cols[0]))?
            .len();

        let mut conn = Connection::open(&sqlite_path)?;

        // Build CREATE TABLE SQL
        let col_defs: Vec<String> = needed_cols.iter().map(|c| format!("\"{}\" REAL", c)).collect();
        let create_sql = format!("CREATE TABLE IF NOT EXISTS metrics ({})", col_defs.join(", "));
        conn.execute(&create_sql, [])?;

        let tx = conn.transaction()?;

        let insert_cols: Vec<String> = needed_cols.iter().map(|c| format!("\"{}\"", c)).collect();

        // Prepare statement with placeholders
        let placeholder_row = format!("({})", vec!["?"; needed_cols.len()].join(","));
        let insert_sql_base = format!("INSERT INTO metrics ({}) VALUES ", insert_cols.join(","));

        for batch_start in (0..n_rows).step_by(batch_size) {
            let batch_end = std::cmp::min(batch_start + batch_size, n_rows);

            // Build params and SQL
            let mut params: Vec<&dyn ToSql> = Vec::with_capacity(needed_cols.len() * (batch_end - batch_start));
            let placeholders: Vec<String> = (batch_start..batch_end)
                .map(|i| {
                    for col in &needed_cols {
                        params.push(&columns[col][i] as &dyn ToSql);
                    }
                    placeholder_row.clone()
                })
                .collect();

            let insert_sql = format!("{}{}", insert_sql_base, placeholders.join(","));
            tx.execute(&insert_sql, params.as_slice())?;
        }

        tx.commit()?;
        Ok(())
    })
    .await??;

    Ok(())
}





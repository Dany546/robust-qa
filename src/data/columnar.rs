use std::collections::HashMap;
use std::error::Error;
use polars::prelude::{DataFrame, Series};
use rusqlite::{Connection, params, ToSql};
use std::path::Path;
use std::fs::File;
use utils::{make_xy_column_names, build_combinations, extract_xy};

/// A simple ColumnarTable wrapper using Polars DataFrame
#[derive(Clone)]
pub struct ColumnarTable {
    pub df: DataFrame,
}

impl ColumnarTable {
    pub fn new(df: DataFrame) -> Self {
        Self { df }
    }

    /// Extract a column as Vec<f64>
    pub fn column_f64(&self, col: &str) -> Option<Vec<f64>> {
        self.df.column(col).ok()?.f64().ok().map(|s| s.into_no_null_iter().collect())
    }

    /// Number of rows
    pub fn len(&self) -> usize {
        self.df.height()
    }
}

/// Load JSON or SQLite cache into ColumnarTable
pub fn load_dataset(
    json_path: &str,
    sqlite_path: &str,
    needed_cols: &[String],
) -> Result<ColumnarTable, Box<dyn Error>> {
    // If SQLite exists, load from it
    if Path::new(sqlite_path).exists() {
        let conn = Connection::open(sqlite_path)?;
        let cols = needed_cols.join(", ");
        let mut stmt = conn.prepare(&format!("SELECT {} FROM metrics", cols))?;
        let mut rows = stmt.query([])?;

        let mut columns: HashMap<String, Vec<f64>> = needed_cols.iter().map(|c| (c.clone(), vec![])).collect();

        while let Some(row) = rows.next()? {
            for (i, col) in needed_cols.iter().enumerate() {
                let val: f64 = row.get(i)?;
                columns.get_mut(col).unwrap().push(val);
            }
        }

        // Convert to Polars DataFrame
        let mut df_cols = Vec::new();
        for col in needed_cols {
            let s = Series::new(col, &columns[col]);
            df_cols.push(s);
        }

        return Ok(ColumnarTable::new(DataFrame::new(df_cols)?));
    }

    // Otherwise, load JSON
    let file = File::open(json_path)?;
    let table: serde_json::Value = serde_json::from_reader(file)?;
    let columns_json = table["columns"].as_array().unwrap();
    let data_json = table["data"].as_array().unwrap();

    // Map needed_cols -> index
    let mut col_indices = Vec::new();
    for (i, c) in columns_json.iter().enumerate() {
        if let Some(col_name) = c.as_str() {
            if needed_cols.contains(&col_name.to_string()) {
                col_indices.push((col_name.to_string(), i));
            }
        }
    }

    // Collect data
    let mut columns: HashMap<String, Vec<f64>> = needed_cols.iter().map(|c| (c.clone(), vec![])).collect();
    for row in data_json {
        let row_arr = row.as_array().unwrap();
        for (col, idx) in &col_indices {
            let val = row_arr.get(*idx).and_then(|v| v.as_f64()).unwrap_or(f64::NAN);
            columns.get_mut(col).unwrap().push(val);
        }
    }

    // Save to SQLite cache
    let conn = Connection::open(sqlite_path)?;
    let col_defs: Vec<String> = needed_cols.iter().map(|c| format!("{} REAL", c)).collect();
    let create_sql = format!("CREATE TABLE IF NOT EXISTS metrics ({})", col_defs.join(", "));
    conn.execute(&create_sql, [])?;

    let insert_sql = format!("INSERT INTO metrics ({}) VALUES ({})",
        needed_cols.join(", "),
        vec!["?"; needed_cols.len()].join(", ")
    );
    let tx = conn.transaction()?;
    for i in 0..columns[needed_cols[0]].len() {
        let vals: Vec<f64> = needed_cols.iter().map(|c| columns[c][i]).collect();
        let params: Vec<&dyn ToSql> = vals.iter().map(|v| v as &dyn ToSql).collect();
        tx.execute(&insert_sql, params.as_slice())?;
    }
    tx.commit()?;

    // Convert to Polars DataFrame
    let mut df_cols = Vec::new();
    for col in needed_cols {
        let s = Series::new(col, &columns[col]);
        df_cols.push(s);
    }

    Ok(ColumnarTable::new(DataFrame::new(df_cols)?))
}

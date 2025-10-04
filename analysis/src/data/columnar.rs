use std::collections::{HashMap, HashSet};
use rusqlite::{Connection, ToSql};
use std::path::Path;
use rayon::prelude::*;
use std::fs::File;
use tokio::task;
use crate::data::sqlite::save_to_sqlite_parallel;

#[derive(Clone)]
pub struct ColumnarTable {
    pub n_rows: usize,
    pub data: HashMap<String, Vec<f64>>,          // scalar columns
    pub vector_data: HashMap<String, Vec<Vec<f64>>>, // per-row vector columns
}

impl ColumnarTable {
    pub fn new(
        n_rows: usize,
        data: HashMap<String, Vec<f64>>,
        vector_data: HashMap<String, Vec<Vec<f64>>>,
    ) -> Self {
        Self { n_rows, data, vector_data }
    }

    /// Return a slice reference to a scalar column, or empty slice if not found
    pub fn column_f64(&self, col: &str) -> &[f64] {
        let out = self.data.get(col).map(|v| v.as_slice()).unwrap_or(&[]);
        if out.iter().any(|v| v.is_nan()) {
            println!("col {} contains NaNs", col);
        }
        out
    }

    /// Return a slice reference to a vector column, or empty slice if not found
    pub fn vector_column(&self, col: &str) -> &[Vec<f64>] {
        self.vector_data.get(col).map(|v| v.as_slice()).unwrap_or(&[])
    }

    pub fn len(&self) -> usize {
        self.n_rows
    }
}

pub fn load_dataset(
    json_path: &str,
    sqlite_path: &str,
    combinations: &Vec<(String, String, f64)>,
) -> Result<ColumnarTable, anyhow::Error> {

    // --- 3. Determine needed columns ---
    let mut needed_cols: Vec<String> = Vec::new();
    let mut seen_y = HashSet::new();
    for (x_col, y_col, _dl) in combinations {
        // x_col is always unique → push directly
        needed_cols.push(x_col.to_string());
        // y_col may repeat → insert into set first
        if seen_y.insert(y_col.clone()) {
            needed_cols.push(y_col.to_string());
        }
    }
    
    // If SQLite exists, load from it
    if Path::new(sqlite_path).exists() {
        println!("[INFO] Loading cached Data from SQLite ...");
        let conn = Connection::open(sqlite_path)?;
        let cols = needed_cols
            .iter()
            .map(|c| format!("\"{}\"", c))
            .collect::<Vec<_>>()
            .join(", "); 
        let mut stmt = conn.prepare(&format!("SELECT {} FROM metrics", cols))?;
        let mut rows = stmt.query([])?;

        let mut columns: HashMap<String, Vec<f64>> =
            needed_cols.iter().map(|c| (c.clone(), Vec::new())).collect();

        while let Some(row) = rows.next()? {
            for (i, col) in needed_cols.iter().enumerate() {
                let val: f64 = row.get(i)?;
                columns.get_mut(col).unwrap().push(val);
            }
        }

        let n_rows = columns[&needed_cols[0]].len();
        return Ok(ColumnarTable::new(n_rows, columns, HashMap::new()));
    }
    println!("[INFO] Loading Data from json ...");

    // Otherwise, load JSON
    let file = File::open(json_path)?;
    let table: serde_json::Value = serde_json::from_reader(file)?;
    let columns_json = table["columns"].as_array().unwrap();
    let data_json = table["data"].as_array().unwrap();

     // Map needed_cols -> index
    let col_indices: Vec<(String, usize)> = needed_cols
        .iter()
        .filter_map(|col| {
            columns_json.iter().position(|c| c.as_str() == Some(col))
                        .map(|idx| (col.clone(), idx))
        })
        .collect();

    if col_indices.len() != needed_cols.len() {
        for col in needed_cols {
            if !col_indices.iter().any(|(c, _)| *c == col) {
                eprintln!("needed column not found in JSON: {}", col);
            }
        }
        panic!("Some needed_cols not found in JSON");
    }
    
    // Parallel collection of columns
    let n_rows = columns_json.len();
    let columns: HashMap<String, Vec<f64>> = col_indices
        .par_iter()
        .map(|(col, idx)| {
            let mut vec: Vec<f64> = data_json
                .par_iter()
                .map(|row| {
                    row.as_array()
                        .and_then(|arr| arr.get(*idx))
                        .and_then(|v| v.as_f64())
                        .unwrap_or(f64::NAN)
                })
                .collect();
            if col.to_lowercase().contains("logitmean") {
                for xs in vec.iter_mut() {
                    *xs = 1.0 / (1.0 + (-*xs).exp());
                }
            }
            (col.clone(), vec)
        })
        .collect(); 

    // Save SQLite cache
    save_to_sqlite_parallel(sqlite_path.to_string(), needed_cols, columns.clone());
    
    Ok(ColumnarTable::new(n_rows, columns, HashMap::new()))
}















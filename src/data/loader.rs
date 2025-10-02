use std::fs;
use serde_json::Value;
use anyhow::Result;
use crate::data::columnar::ColumnarTable;
use crate::data::sqlite::{load_traces_sqlite, save_traces_sqlite};

/// Load JSON dataset into ColumnarTable, cache as SQLite
pub fn load_dataset(json_path: &str, sqlite_path: &str, needed_cols: &[&str]) -> Result<ColumnarTable> {
    if std::path::Path::new(sqlite_path).exists() {
        // Load from cached SQLite
        return Ok(ColumnarTable::load_sqlite(sqlite_path, needed_cols)?);
    }

    // Load JSON table
    let file = fs::File::open(json_path)?;
    let v: Value = serde_json::from_reader(file)?;
    let columns = v["columns"].as_array().unwrap();
    let data = v["data"].as_array().unwrap();

    let mut ct_data: HashMap<String, Vec<f64>> = HashMap::new();
    let mut col_indices = Vec::new();

    for (i, col) in columns.iter().enumerate() {
        let col_name = col.as_str().unwrap();
        if needed_cols.contains(&col_name) {
            col_indices.push((col_name.to_string(), i));
            ct_data.insert(col_name.to_string(), Vec::with_capacity(data.len()));
        }
    }

    for row in data {
        let row_arr = row.as_array().unwrap();
        for (col_name, idx) in &col_indices {
            let val = row_arr[*idx].as_f64().unwrap_or(f64::NAN);
            ct_data.get_mut(col_name).unwrap().push(val);
        }
    }

    let ct = ColumnarTable { n_rows: data.len(), data: ct_data };

    // Save SQLite cache for next run
    ct.save_sqlite(sqlite_path)?;

    Ok(ct)
}

use std::collections::{HashMap, HashSet};
use std::fs::File;
use serde::{Serialize, Deserialize}; 
use crate::data::utils::make_xy_column_names;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TraceData {
    pub method: String,
    pub dataset: String,
    pub drop_level: f64,
    pub fnr: f64,
    pub metric: String,              
    pub aggregation: String,         
    pub metric_aggregation: String,  
    pub th: Vec<f64>,                
    pub ys: Vec<f64>,                
    pub xs: Vec<f64>,                
}

#[derive(Deserialize)]
struct JsonTable {
    columns: Vec<String>,
    data: Vec<Vec<f64>>,
}

/// Load JSON dataset and only keep relevant columns
pub fn load_dataset_json(
    dataset: &str,
    pref: &str,
    dir_path: &str,
    combos: &[(String, String, f64, String, String, String)]
) -> anyhow::Result<Vec<HashMap<String,f64>>> {
    let needed_cols: HashSet<String> = combos.iter().filter(|(d, ..)| d == dataset)
        .flat_map(|(_, method, drop_level, metric, agg, met_agg)| {
            let (x_col, y_col) = make_xy_column_names(method, *drop_level, metric, agg, met_agg, pref);
            vec![x_col,y_col]
        }).collect();

    let mut all_rows = Vec::new();

    for split in ["validation"] {
        let path = format!("{}/{}_Metrics_{}_{}.table.json", dir_path, dataset, pref, split);
        let file = File::open(&path)?;
        let table: JsonTable = serde_json::from_reader(file)?;

        let mut col_indices = Vec::new();
        for (i,col) in table.columns.iter().enumerate() {
            if needed_cols.contains(col) { col_indices.push((col.clone(),i)); }
        }

        for row in table.data {
            let mut map = HashMap::with_capacity(col_indices.len());
            for (col,idx) in &col_indices {
                map.insert(col.clone(), *row.get(*idx).unwrap_or(&f64::NAN));
            }
            all_rows.push(map);
        }
    }

    Ok(all_rows)
}

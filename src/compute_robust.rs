mod config;
mod data;
mod metrics;
mod plot;
mod trace;

use rand::{Rng};
use serde::{Serialize, Deserialize};
use std::fs::File;
use serde_json::Value;
use std::error::Error;
use rayon::prelude::*;
use itertools::iproduct;
use rand::SeedableRng;         
use rand::rngs::StdRng;   
use std::io::{self}; 
use std::path::Path;
use std::collections::{HashSet, HashMap}; 
use data::loader::{TraceData};
use data::columnar::{load_dataset};
use data::utils::get_xy_for_method; 
use plot::{save_traces_json, save_traces_html};

fn main() -> anyhow::Result<()> {
    // --- 1. Config ---
    let datasets = vec!["Brats_last_final", "LUNG_last_final", "ABDO1k_last_final", "pancreas_last_final"];
    let methods = vec!["TTA", "MCd", "ckp-DE", "DE", "OOD"];
    let fnrs: Vec<f64> = (1..6).map(|i| i as f64 / 20.0).collect();
    let quality_thresholds: Vec<f64> = (0..20).map(|i| 0.6 + 0.02*i as f64).collect();
    let quantiles: Vec<f64> = (0..40).map(|i| 0.025 + 0.025*i as f64).collect();
    let dir_path = "..";
    let pref = "49";

    
    let traces_sqlite_path = format!("{}/precomputed_tables.db", dir_path);

    let all_data: Vec<TraceData> = if Path::new(&traces_sqlite_path).exists() {
        println!("[INFO] Loading cached TraceData from SQLite...");
        load_traces_sqlite(&traces_sqlite_path)?
    } else {
        // --- 2. Build combinations ---
        let combinations = trace::build_combinations(&datasets, &methods);
    
        // --- 3. Determine needed columns ---
        let needed_cols: Vec<String> = combinations.iter()
            .flat_map(|(dataset, method, dl, metric, agg, met_agg)| {
                let (x_col, y_col) = trace::make_xy_column_names(method, *dl, metric, agg, met_agg, pref);
                vec![x_col, y_col]
            })
            .collect();
    
        // --- 4. Load all datasets into ColumnarTables (SQLite cache) ---
        let mut all_data_ct = HashMap::new();
        for dataset in &datasets {
            let json_path = format!("{}/{}_Metrics_{}.table.json", dir_path, dataset, pref);
            let sqlite_path = format!("{}/{}_metrics.db", dir_path, dataset);
            let ct = load_dataset(&json_path, &sqlite_path, &needed_cols)?;
            all_data_ct.insert(dataset.to_string(), ct);
        } 
        // Save to SQLite cache
        save_traces_sqlite(&all_data_ct, &traces_sqlite_path)?;
        all_data_ct
    };

    // --- 5. Compute all TraceData in parallel ---
    let out_data: Vec<TraceData> = combinations.par_iter()
        .flat_map(|(dataset, method, dl, metric, agg, met_agg)| {
            let ct = all_data.get(dataset).unwrap();
            let mut local_traces = Vec::new();

            if let Some((x, y)) = get_xy_for_method(ct, method, *dl, metric, agg, met_agg, pref) {
                for &fnr in &fnrs {
                    // Call your optimized precision computation
                    let (prec_curves, thresholds) =
                        metrics::precision::precision_at_robust(&x, &y, &quality_thresholds, &quantiles, 2000, "mean", metric, fnr);

                    local_traces.push(TraceData {
                        method: method.to_string(),
                        dataset: dataset.to_string(),
                        fnr,
                        drop_level: *dl,
                        metric: metric.to_string(),
                        aggregation: agg.to_string(),
                        metric_aggregation: met_agg.to_string(),
                        xs: quality_thresholds.clone(),
                        th: thresholds.clone(),
                        ys: prec_curves.clone(),
                    });
                }
            }
            local_traces
        })
        .collect();

    // --- 6. Export results ---
    save_traces_json(&out_data, "precomputed.json")?;
    save_traces_html(&out_data, "plot.html")?;
    save_traces_sqlite(&out_data, &format!("{}/precomputed_traces.db", dir_path))?;

    println!("[INFO] Completed pipeline. JSON + HTML saved.");
    Ok(())
}


pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    main()?;
    Ok(())
}
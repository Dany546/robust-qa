
use rayon::prelude::*;
use std::path::Path;
use std::collections::{HashMap, HashSet}; 
use analysis::data::columnar::{load_dataset};
use analysis::data::utils::{get_xy_for_method, build_combinations, make_xy_column_names, make_names}; 
use analysis::data::sqlite::{load_traces_sqlite, save_traces, TraceRow};
use analysis::metrics::precision::tp_at_robust;
use analysis::data::columnar::ColumnarTable;
use std::sync::{Arc, Mutex};
use tokio;
use std::sync::mpsc;
use indicatif::{ProgressBar, ProgressStyle};
use std::sync::atomic::{AtomicUsize, Ordering};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // --- 1. Config ---
    let datasets = vec!["Brats_last_final", "LUNG_last_final", "ABDO1k_last_final", "pancreas_last_final"];
    let dir_path = "/auto/home/users/d/a/darimez/MIRO/UNET/data_analysis";
    let pref = "49";
    
    let fnrs: Vec<f64> = (1..5).map(|i| i as f64 / 50.0).collect();
    let quality_thresholds: Vec<f64> = (0..39).map(|i| 0.6 + 0.01*i as f64).collect();
    let quantiles: Vec<f64> = (0..39).map(|i| 0.025 + 0.025*i as f64).collect();
 
    // --- 2. Build combinations ---
    let combinations_tuple = build_combinations();
    let combinations = make_names(&combinations_tuple, &pref);
    
    for dataset in &datasets {
        let sqlite_path = format!("{}/{}_table.db", dir_path, dataset);
        let json_path = format!("{}/{}_Metrics_{}_validation.table.json", dir_path, dataset, pref);
        let trace_path = format!("{}/{}_traces.db", dir_path, dataset);
    
        let all_data: ColumnarTable = load_dataset(&json_path, &sqlite_path, &combinations).await?;
        println!("[INFO] Loaded Data ...");
        
        // Shared atomic counter for progress
        let total = combinations.len();
        let pb = ProgressBar::new(total as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("[{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
                .unwrap()
                .progress_chars("##-"),
        );
        println!("[INFO] Running ...");
        
        let (tx, rx) = mpsc::channel::<TraceRow>();
        let writer_handle = std::thread::spawn(move || save_traces(rx, trace_path));

        combinations.par_iter().for_each(|(x_col, y_col, dl)| {
            let x = all_data.column_f64(x_col);
            let y = all_data.column_f64(y_col);
    
            for &fnr in &fnrs {
                let (prec_curves, thresholds) =
                    tp_at_robust(&x, &y, &quality_thresholds, &quantiles, 2000, "mean", y_col, fnr);
                
                tx.send(TraceRow {
                    x_col: x_col.clone(),
                    y_col: y_col.clone(),
                    fnr,
                    drop_level: *dl,
                    th: thresholds,
                    xs: quality_thresholds.clone(),
                    ys: prec_curves,
                })
                .unwrap();
            }
            pb.inc(1);
        });
    
        drop(tx);
        writer_handle.join().unwrap()?;
    }
    println!("[INFO] Completed pipeline. JSON + HTML saved.");
    Ok(())
}


pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    main()?;
    Ok(())
}
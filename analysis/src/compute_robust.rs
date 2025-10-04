
use rayon::prelude::*;
use std::path::Path;
use std::collections::{HashMap, HashSet}; 
use analysis::data::columnar::{load_dataset};
use analysis::data::utils::{get_xy_for_method, build_combinations, make_xy_column_names, make_names}; 
use analysis::data::sqlite::{load_traces_sqlite, save_traces_sqlite};
use analysis::metrics::precision::precision_at_robust;
use analysis::data::columnar::ColumnarTable;
use std::sync::{Arc, Mutex};
use tokio;
use indicatif::{ProgressBar, ProgressStyle};
use std::sync::atomic::{AtomicUsize, Ordering};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // --- 1. Config ---
    let datasets = vec!["Brats_last_final", "LUNG_last_final", "ABDO1k_last_final", "pancreas_last_final"];
    let dir_path = "/auto/home/users/d/a/darimez/MIRO/UNET/data_analysis";
    let pref = "49";
    
    let fnrs: Vec<f64> = (1..6).map(|i| i as f64 / 20.0).collect();
    let quality_thresholds: Vec<f64> = (0..20).map(|i| 0.6 + 0.02*i as f64).collect();
    let quantiles: Vec<f64> = (0..40).map(|i| 0.025 + 0.025*i as f64).collect();
 
    // --- 2. Build combinations ---
    let combinations_tuple = build_combinations();
    let combinations = make_names(&combinations_tuple, &pref);
    
    for dataset in &datasets {
        let sqlite_path = format!("{}/{}_table.db", dir_path, dataset);
        let json_path = format!("{}/{}_Metrics_{}_validation.table.json", dir_path, dataset, pref);
    
        let all_data: ColumnarTable = load_dataset(&json_path, &sqlite_path, &combinations)?;
        println!("[INFO] Loaded Data ...");
        
        let mut scalar_cols: HashMap<String, Vec<f64>> = HashMap::new();
        scalar_cols.insert("drop_level".into(), Vec::new());
        scalar_cols.insert("fnr".into(), Vec::new());
        
        let xs_vecs: Vec<Vec<f64>> = Vec::new();
        let th_vecs: Vec<Vec<f64>> = Vec::new();
        let ys_vecs: Vec<Vec<f64>> = Vec::new();
    
        // Wrap scalar_cols
        let scalar_cols = Arc::new(Mutex::new(scalar_cols));
        
        // Wrap vector-of-vectors
        let xs_vecs = Arc::new(Mutex::new(xs_vecs));
        let th_vecs = Arc::new(Mutex::new(th_vecs));
        let ys_vecs = Arc::new(Mutex::new(ys_vecs));
        
        // Shared atomic counter for progress
        let progress = Arc::new(AtomicUsize::new(0));
        let total = combinations.len();
        let pb = ProgressBar::new(total as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("[{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
                .unwrap()
                .progress_chars("##-"),
        );
        println!("[INFO] Running ...");
        combinations.par_iter().take(2).for_each(|(x_col, y_col, dl): &(String, String, f64)| { 
            let x = all_data.column_f64(&x_col);
            let y = all_data.column_f64(&y_col); 
            /// println!("{:?}", x);
            for &fnr in &fnrs {
                let (prec_curves, thresholds) = precision_at_robust(
                    &x, &y, &quality_thresholds, &quantiles, 2000, "mean", &y_col, fnr
                );
                println!("{:?} {:?} {:?}", x_col, fnr, prec_curves);
                // Lock and push safely
                {
                    let mut sc = scalar_cols.lock().unwrap();
                    sc.get_mut("drop_level").unwrap().push(*dl);
                    sc.get_mut("fnr").unwrap().push(fnr);
                }
                {
                    let mut xs = xs_vecs.lock().unwrap();
                    xs.push(quality_thresholds.clone());
                }
                {
                    let mut th = th_vecs.lock().unwrap();
                    th.push(thresholds.clone());
                }
                {
                    let mut ys = ys_vecs.lock().unwrap();
                    ys.push(prec_curves.clone());
                }
            } 
            // Update progress
            pb.inc(1);
        });
    
        // Lock the Arc<Mutex<…>> to get the inner HashMap
        let scalar_cols_locked = Arc::try_unwrap(scalar_cols)
            .expect("Arc has multiple owners")
            .into_inner()
            .expect("Mutex poisoned");
        
        let xs_vecs_locked = Arc::try_unwrap(xs_vecs)
            .expect("Arc has multiple owners")
            .into_inner()
            .expect("Mutex poisoned");
        
        let th_vecs_locked = Arc::try_unwrap(th_vecs)
            .expect("Arc has multiple owners")
            .into_inner()
            .expect("Mutex poisoned");
        
        let ys_vecs_locked = Arc::try_unwrap(ys_vecs)
            .expect("Arc has multiple owners")
            .into_inner()
            .expect("Mutex poisoned");
        
        // Build the ColumnarTable
        let n_rows = scalar_cols_locked
            .values()
            .next()
            .map(|v| v.len())
            .unwrap_or(0);
        
        let mut vector_cols = HashMap::new();
        vector_cols.insert("xs".to_string(), xs_vecs_locked);
        vector_cols.insert("th".to_string(), th_vecs_locked);
        vector_cols.insert("ys".to_string(), ys_vecs_locked);
        
        let out_data = ColumnarTable::new(n_rows, scalar_cols_locked, vector_cols);
    
        // --- 6. Export results ---
        // save_traces_json(&out_data, "precomputed.json")?;
        // save_traces_html(&out_data, "plot.html")?;
        save_traces_sqlite(&out_data, &combinations_tuple, pref, &format!("{}/{}_traces.db", dir_path, dataset)).await?;
    }
    println!("[INFO] Completed pipeline. JSON + HTML saved.");
    Ok(())
}


pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    main()?;
    Ok(())
}
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
use std::collections::{HashSet, HashMap};
use interp1d::{Interp1d, error::InterpError};


fn compute_precision_fnr(
xb: &[f64], 
yb: &[f64], 
thy: f64, 
quantiles: &[f64]
) -> (Vec<f64>, Vec<f64>) {

    let mut precision = Vec::with_capacity(quantiles.len());
    let mut fnr = Vec::with_capacity(quantiles.len());

    for &th in quantiles {
        let n_y_high = yb.iter().filter(|&&v| v >= thy && v < 1.0).count();
        
        if n_y_high == 0 {
            precision.push(1.0);
            fnr.push(1.0);
        } else {
            let tp = xb.iter().zip(yb.iter())
                .filter(|(&xi, &yi)| xi >= th && yi >= thy)
                .count();

            let fn_val = xb.iter().zip(yb.iter())
                .filter(|(&xi, &yi)| xi >= th && yi < thy)
                .count();

            precision.push(tp as f64 / yb.len() as f64);
            fnr.push(fn_val as f64 / yb.len() as f64);
        }
    }
    
    (precision, fnr)
}

// aggregation of bootstrapped statistics, either mean or interpolated quantile 
fn aggregate_curves(curves: &[Vec<f64>], method: &str, percentile: f64, len: usize) -> Vec<f64> {
    let n = curves[0].len();

    match method {
        "mean" => (0..n).into_par_iter()
            .map(|i| curves.iter().map(|v| v[i]).sum::<f64>() / len as f64)
            .collect(),

        "percentile" => (0..n).into_par_iter()
            .map(|i| {
                let mut col: Vec<f64> = curves.iter().map(|v| v[i]).collect();
                col.sort_by(|a, b| a.partial_cmp(b).unwrap());
                let m = col.len();
                if m == 1 {
                    return col[0];
                }
                let h = (m - 1) as f64 * percentile.clamp(0.0, 1.0);
                let i0 = h.floor() as usize;
                let i1 = h.ceil() as usize;
                let frac = h - i0 as f64;
                (1.0 - frac) * col[i0] + frac * col[i1]
            })
            .collect(),

        _ => panic!("Unknown aggregation method"),
    }
}

// unsafe interpolation for fast compute
fn linear_interpolate(x: &[f64], y: &[f64], xi: f64, n: usize) -> Result<f64, InterpError> {

    // Below range 
    if xi <= x[0] {
        return Ok(f64::NAN);
    }

    // Above range → clamp to last value
    if xi >= x[n - 1] {
        return Ok(y[n - 1]);
    }

    let interp = Interp1d::new_sorted(x.to_vec(), y.to_vec())?;
    let yi = interp.interpolate(xi);
    
    Ok(yi)
}

fn bootstrap_sample(x: &[f64], y: &[f64], n: usize, rng: &mut impl Rng) -> (Vec<f64>, Vec<f64>) { 

    let mut xb = Vec::with_capacity(n);
    let mut yb = Vec::with_capacity(n);

    for _ in 0..n {
        let idx = rng.gen_range(0..n);
        xb.push(x[idx]);
        yb.push(y[idx]);
    }

    (xb, yb)
}

fn precision_at_robust(
    x: &[f64], 
    y: &[f64], 
    quality_thresholds: &[f64],  
    quantiles: &[f64],  
    n_bootstrap: usize, 
    method: &str,
    metric: &str,
    q: f64, // the target FNR value for interpolation
) -> (Vec<f64>, Vec<f64>) {

    let mut all_precision: Vec<f64> = Vec::with_capacity(quality_thresholds.len());
    let mut all_thx: Vec<f64> = Vec::with_capacity(quality_thresholds.len());
    let n = quantiles.len();
    let conf = if metric.to_lowercase().contains("adpl") {
        0.05 // 5th quantile
    } else {
        0.95 // 95th quantile
    };
    
    for &thy in quality_thresholds {
    
        let (precision_curves, fnr_curves): (Vec<_>, Vec<_>) = (0..n_bootstrap)
            .into_par_iter()  // parallel iterator
            .map(|i| {
                let mut rng = StdRng::seed_from_u64(i as u64);
                let (xb, yb) = bootstrap_sample(x, y, n, &mut rng);
                compute_precision_fnr(&xb, &yb, thy, &quantiles)
            })
            .unzip();

        let precision_agg = aggregate_curves(&precision_curves, method, 1.0 - conf, n_bootstrap);
        let fnr_agg = aggregate_curves(&fnr_curves, method, conf, n_bootstrap);
        
        println!("[DEBUG] fnr: {:?}", fnr_agg)
        // --- Interpolation step ---
        let (thx, precision_at_q) = (|| -> Result<(f64, f64), InterpError> {
            let th_val = linear_interpolate(&fnr_agg, &quantiles, q, n);
            let prec_val = linear_interpolate(&fnr_agg, &precision_agg, q, n);
            Ok((th_val?, prec_val?))
        })().unwrap_or_else(|_: InterpError| (f64::NAN, f64::NAN));

        all_precision.push(precision_at_q);
        all_thx.push(thx);
    }
    (all_precision, all_thx)
}

fn make_xy_column_names(
    method: &str,
    drop_level: f64,
    metric: &str,
    agg: &str,
    met_agg: &str,
    pref: &str,
) -> (String, String) {
    match method {
        "MCd" => (
            format!("tumor_paired{}MCd_{:.1}_seg{}_{}_{}", metric, drop_level, agg, met_agg, pref),
            format!("tumor_{}_{:.1}_seg{}_UQ_meanMC_{}", metric, drop_level, agg, pref),
        ),
        "ckp-DE" => (
            format!("tumor_paired{}DE_{:.1}_seg{}_{}_{}", metric, drop_level, agg, met_agg, pref),
            format!("tumor_{}_{:.1}_seg{}_UQ_meanDE_{}", metric, drop_level, agg, pref),
        ),
        "DE" => (
            format!("tumor_paired{}DE_DE{}_{}_{}", metric, agg, met_agg, pref),
            format!("tumor_{}_DE{}_UQ_meanDE_{}", metric, agg, pref),
        ),
        "TTA" => (
            format!("tumor_paired{}MCd_{:.1}_flip_seg{}_{}_{}", metric, drop_level, agg, met_agg, pref),
            format!("tumor_{}_{:.1}_flip_seg{}_UQ_meanMC_{}", metric, drop_level, agg, pref),
        ),
        "OOD" => (
            format!("tumor_{}_diff_2_{:.1}_30", metric, drop_level),
            format!("tumor_{}_{:.1}_30", metric, drop_level),
        ),
        _ => panic!("Unknown method '{}'", method),
    }
}

#[derive(Deserialize)]
struct JsonTable {
    columns: Vec<String>,
    data: Vec<Vec<f64>>,
}

/// Load JSON dataset and only keep relevant columns
fn load_dataset_json(
    dataset: &str,
    pref: &str,
    dir_path: &str,
    combos: &[(String, String, f64, String, String, String)],
) -> Result<Vec<HashMap<String, f64>>, Box<dyn Error>> {
    // Collect relevant column names for this dataset
    let needed_cols: HashSet<String> = combos
        .iter()
        .filter(|(d, ..)| d == dataset)
        .flat_map(|(_, method, drop_level, metric, agg, met_agg)| {
            let (x_col, y_col) = make_xy_column_names(method, *drop_level, metric, agg, met_agg, pref);
            vec![x_col, y_col]
        })
        .collect();

    let mut all_rows = Vec::new();

    for split in ["validation"] {
        let path = format!("{}/{}_Metrics_{}_{}.table.json", dir_path, dataset, pref, split);
        let file = File::open(&path)?;
        let table: JsonTable = serde_json::from_reader(file)?; // much faster than Value

        // Precompute indices of needed columns
        let mut col_indices = Vec::new();
        for (i, col) in table.columns.iter().enumerate() {
            if needed_cols.contains(col) {
                col_indices.push((col.clone(), i));
            }
        }

        // Collect rows (no Rayon here → JSON already deserialized)
        // Still much faster than per-row HashMap cloning
        for row in table.data {
            let mut map = HashMap::with_capacity(col_indices.len());
            for (col, idx) in &col_indices {
                let val = *row.get(*idx).unwrap_or(&f64::NAN);
                map.insert(col.clone(), val);
            }
            all_rows.push(map);
        }
    }

    Ok(all_rows)
}


/// Extract x and y columns for a given dataset and method
fn extract_xy(
    all_data_json: &HashMap<String, Vec<HashMap<String, f64>>>,
    dataset: &str,
    x_cols: &[&str],
    y_cols: &[&str],
) -> (Vec<f64>, Vec<f64>) {
    let mut xs = Vec::new();
    let mut ys = Vec::new();
    
    // Look up dataset
    if let Some(rows) = all_data_json.get(dataset) {
        for row in rows {
            let mut include_row = true;
            let mut x_row = Vec::new();
            let mut y_row = Vec::new();
        
            for &x_col in x_cols {
                if let Some(&val) = row.get(x_col) {
                    x_row.push(val);
                } else {
                    panic!("[DEBUG] Missing x_col '{}'", x_col);
                }
            }
        
            for &y_col in y_cols {
                if let Some(&val) = row.get(y_col) {
                    y_row.push(val);
                } else {
                    panic!("[DEBUG] Missing y_col '{}' ", y_col);
                }
            }
        
            if include_row {
                if x_row.len() == 1 && y_row.len() == 1 {
                    xs.push(x_row[0]);
                    ys.push(y_row[0]);
                } else {
                    xs.extend(x_row);
                    ys.extend(y_row);
                }
            }
        }

    }
    
    (xs, ys)
}


/// Get x and y vectors for a given dataset, method, drop_level, and metric
fn get_xy_for_method(
    all_data_json: &HashMap<String, Vec<HashMap<String, f64>>>,
    dataset: &str,
    method: &str,
    drop_level: f64,
    metric: &str,
    aggregation_aggregation: &str, // only for MCd/DE/TTA
    metric_aggregation: &str,      // only for MCd/DE/TTA, x_col only
    pref: &str,
) -> Option<(Vec<f64>, Vec<f64>)> {

    let (x_col, y_col) = make_xy_column_names(&method, drop_level, &metric, &aggregation_aggregation, &metric_aggregation, pref); 
    Some(extract_xy(all_data_json, dataset, &[&x_col], &[&y_col]))
}

fn build_combinations(
    datasets: &Vec<&str>,
    methods: &Vec<&str>,
) -> Vec<(String, String, f64, String, String, String)> {
    // Example vectors 
    let drop_levels_mcd: &[f64] = &[0.1, 0.2, 0.3, 0.4, 0.5];
    let drop_levels_others: &[f64] = &[0.0, 0.1, 0.2, 0.3, 0.4, 0.5];
    let drop_levels_de: &[f64] = &[0.0];
    let metrics: &[&str] = &["dice", "sdice"];
    let aggregations: &[&str] = &["", "_union", "_inter", "_consensus", "_logit"];
    let metric_aggs: &[&str] = &["mean", "max", "min", "logitmean"];


    datasets.iter().flat_map(|_dataset| {
        let dataset = _dataset;
        let aggregations = aggregations;

        methods.iter().flat_map(move |method| {
            let method = method;

            let drop_levels = match *method {
                "TTA" | "OOD" => &drop_levels_others,
                "MCd" => &drop_levels_mcd,
                _ => &drop_levels_de,
            };

            iproduct!(
                drop_levels.iter().copied(),
                metrics.iter().copied(),
                aggregations.iter().copied(),
                metric_aggs.iter().copied()
            )
            .map(move |(dl, metric, agg, met_agg)| {
                (
                    dataset.to_string(),
                    method.to_string(),
                    dl,
                    metric.to_string(),
                    agg.to_string(),
                    met_agg.to_string(),
                )
            })
        })
    }).collect()
}

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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let datasets = vec!["Brats_last_final", "LUNG_last_final", "ABDO1k_last_final", "pancreas_last_final"];
    let methods = vec!["TTA", "MCd", "ckp-DE", "DE", "OOD"];
    let fnrs: Vec<f64> = (1..6).map(|i| i as f64 / 20.0).collect();
    let quality_thresholds: Vec<f64> = (0..20).map(|i| 0.6 + 0.02 * i as f64).collect();
    let quantiles: Vec<f64> = (0..40).map(|i| 0.025 + 0.025 * i as f64).collect();
    let dir_path = "..";
    let pref = "49";

    let combinations = build_combinations(&datasets, &methods);
    
    println!("[INFO] Loading data");
    // Load all JSON data once
    let all_data_json: HashMap<_,_> = datasets.par_iter()
    .map(|dataset| {
        let rows = load_dataset_json(dataset, pref, dir_path, &combinations).unwrap();
        (dataset.to_string(), rows)
    })
    .collect();
    println!("[INFO] Loaded data");

    let all_data: Vec<TraceData> = combinations
    .par_iter()
    .flat_map(|(dataset, method, dl, metric, agg, met_agg)| {
        let mut local = Vec::new();

        if let Some((x, y)) = get_xy_for_method(
            &all_data_json,
            dataset,
            method,
            *dl,
            metric,
            agg,
            met_agg,
            pref,
        ) {
            for &fnr in &fnrs {
                let (prec_curves, thresholds) =
                    precision_at_robust(&x, &y, &quality_thresholds, &quantiles, 2000, "mean", metric, fnr);
                
                // Debug: detect degenerate curves
                if prec_curves.iter().all(|&v| (v - 1.0).abs() < f64::EPSILON) {
                    eprintln!("[WARN] precision curve is all 1.0 for dataset={} method={} fnr={} dl={} metric={} agg={} met_agg={}",
                        dataset, method, fnr, dl, metric, agg, met_agg);
                }
                if thresholds.iter().all(|&v| (v - 1.0).abs() < f64::EPSILON) {
                    eprintln!("[WARN] threshold curve is all 1.0 for dataset={} method={} fnr={} dl={} metric={} agg={} met_agg={}",
                        dataset, method, fnr, dl, metric, agg, met_agg);
                }

                local.push(TraceData {
                    method: method.to_string(),
                    dataset: dataset.to_string(),
                    fnr,
                    drop_level: *dl,
                    metric: metric.to_string(),
                    aggregation: agg.to_string(),
                    metric_aggregation: met_agg.to_string(),
                    th: thresholds.clone(),
                    xs: quality_thresholds.clone(),
                    ys: prec_curves.clone(),
                });
            }
        }

        local // this Vec will be collected safely per thread
    })
    .collect();

    // Save to JSON
    let json_str = serde_json::to_string_pretty(&all_data)?;
    std::fs::write("precomputed.json", json_str)?;

    println!("? Saved precomputed results to precomputed.json");
    Ok(())
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    main()?;
    Ok(())
}




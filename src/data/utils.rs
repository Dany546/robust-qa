use std::collections::{HashMap, HashSet};
use std::fs::File;
use serde::{Serialize, Deserialize};
use itertools::iproduct;

use crate::data::columnar::ColumnarTable;


/// Build column names for x and y
pub fn make_xy_column_names(
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

/// Build all combinations
pub fn build_combinations(
    datasets: &[&str],
    methods: &[&str],
) -> Vec<(String, String, f64, String, String, String)> {
    let drop_levels_mcd: &[f64] = &[0.1,0.2,0.3,0.4,0.5];
    let drop_levels_others: &[f64] = &[0.0,0.1,0.2,0.3,0.4,0.5];
    let drop_levels_de: &[f64] = &[0.0];
    let metrics = ["dice","sdice"];
    let aggregations = ["","_union","_inter","_consensus","_logit"];
    let metric_aggs = ["mean","max","min","logitmean"];

    datasets.iter().flat_map(|&dataset| {
        methods.iter().flat_map(move |&method| {
            let drop_levels = match method {
                "TTA" | "OOD" => drop_levels_others,
                "MCd" => drop_levels_mcd,
                _ => drop_levels_de,
            };
            iproduct!(drop_levels.iter().copied(), metrics.iter().copied(), aggregations.iter().copied(), metric_aggs.iter().copied())
                .map(move |(dl, metric, agg, met_agg)| (dataset.to_string(), method.to_string(), dl, metric.to_string(), agg.to_string(), met_agg.to_string()))
        })
    }).collect()
}

/// Extract x and y
pub fn extract_xy(
    all_data_json: &HashMap<String, Vec<HashMap<String,f64>>>,
    dataset: &str,
    x_cols: &[&str],
    y_cols: &[&str]
) -> (Vec<f64>, Vec<f64>) {
    let mut xs = Vec::new();
    let mut ys = Vec::new();
    if let Some(rows) = all_data_json.get(dataset) {
        for row in rows {
            let x_row: Vec<f64> = x_cols.iter().map(|&c| *row.get(c).unwrap()).collect();
            let y_row: Vec<f64> = y_cols.iter().map(|&c| *row.get(c).unwrap()).collect();
            if x_row.len()==1 && y_row.len()==1 {
                xs.push(x_row[0]);
                ys.push(y_row[0]);
            } else {
                xs.extend(x_row);
                ys.extend(y_row);
            }
        }
    }
    (xs, ys)
}

/// Extract x and y vectors from ColumnarTable
pub fn get_xy_for_method(
    ct: &ColumnarTable,
    method: &str,
    drop_level: f64,
    metric: &str,
    aggregation: &str,
    metric_agg: &str,
    pref: &str,
) -> Option<(Vec<f64>, Vec<f64>)> {
    use crate::data::loader::make_xy_column_names;

    let (x_col, y_col) = make_xy_column_names(method, drop_level, metric, aggregation, metric_agg, pref);
    let x = ct.column_f64(&x_col)?;
    let y = ct.column_f64(&y_col)?;
    Some((x, y))
}

/// Get x and y vectors for a dataset/method/drop_level/metric combo
pub fn get_xy_for_method_row_wise(
    ct: &ColumnarTable,
    method: &str,
    drop_level: f64,
    metric: &str,
    aggregation_aggregation: &str,
    metric_aggregation: &str,
    pref: &str
) -> Option<(Vec<f64>, Vec<f64>)> {
    use crate::trace::make_xy_column_names;

    let (x_col, y_col) = make_xy_column_names(method, drop_level, metric, aggregation_aggregation, metric_aggregation, pref);
    Some(extract_xy_columnar(ct, &[&x_col], &[&y_col]))
}
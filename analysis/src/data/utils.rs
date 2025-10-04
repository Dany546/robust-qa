use itertools::iproduct;
use std::collections::{HashSet}; 

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
            format!("tumor_{}__UQ_meanDE_{:.1}_seg{}_{}", metric, drop_level, agg, pref),
        ),
        "DE" => (
            format!("tumor_paired{}DE_{:.1}_DE{}_{}_{}", metric, drop_level, agg, met_agg, pref),
            format!("tumor_{}__UQ_meanDE_{:.1}_DE{}_{}", metric, drop_level, agg, pref),
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

/// Build abstract combinations of method + parameters
pub fn build_combinations() -> Vec<(String, f64, String, String, String)> {
    let drop_levels_mcd: &[f64] = &[0.1, 0.2, 0.3, 0.4, 0.5];
    let drop_levels_others: &[f64] = &[0.0, 0.1, 0.2, 0.3, 0.4, 0.5];
    let drop_levels_de: &[f64] = &[0.0];
    let metrics: &[&str] = &["dice", "sdice"];
    let aggregations: &[&str] = &["", "_union", "_inter", "_consensus", "_logit"];
    let metric_aggs: &[&str] = &["mean", "max", "min", "logitmean"];
    let methods = vec!["TTA", "MCd", "ckp-DE", "DE", "OOD"];

    methods
        .iter()
        .flat_map(move |&method| {
            let drop_levels = match method {
                "TTA" | "OOD" => drop_levels_others,
                "MCd" => drop_levels_mcd,
                _ => drop_levels_de,
            };
            let (aggs, met_aggs): (&[&str], &[&str]) = if method == "OOD" {
                (&[""], &[""])
            } else {
                (aggregations, metric_aggs)
            };

            iproduct!(drop_levels.iter().copied(), metrics.iter(), aggs.iter(), met_aggs.iter())
                .map(move |(dl, metric, agg, met_agg)| {
                    (
                        method.to_string(),
                        dl,
                        metric.to_string(),
                        agg.to_string(),
                        met_agg.to_string(),
                    )
                })
        })
        .collect()
}

/// Turn abstract combinations into concrete x/y column names
pub fn make_names(
    combinations: &[(String, f64, String, String, String)],
    pref: &str,
) -> Vec<(String, String, f64)> {
    combinations
        .iter()
        .map(|(method, dl, metric, agg, met_agg)| {
            let (x, y) = make_xy_column_names(method, *dl, metric, agg, met_agg, pref);
            (x, y, *dl)
        })
        .collect()
}

/// Extract x and y slices from ColumnarTable
pub fn get_xy_for_method<'a>(
    ct: &'a ColumnarTable,
    method: &str,
    drop_level: f64,
    metric: &str,
    aggregation: &str,
    metric_agg: &str,
    pref: &str,
) -> (&'a [f64], &'a [f64]) {

    let (x_col, y_col) = make_xy_column_names(method, drop_level, metric, aggregation, metric_agg, pref);
    let x = ct.column_f64(&x_col); // returns &[f64]
    let y = ct.column_f64(&y_col); // returns &[f64]

    (x, y)
}


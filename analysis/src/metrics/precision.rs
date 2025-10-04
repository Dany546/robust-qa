use rayon::prelude::*;
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use interp1d::{Interp1d, error::InterpError};

/// Compute precision and false negative rate for a threshold
pub fn compute_precision_fnr(
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
            let tp = xb.iter().zip(yb.iter()).filter(|(&xi, &yi)| xi >= th && yi >= thy && yi < 1.0).count();
            let fn_val = xb.iter().zip(yb.iter()).filter(|(&xi, &yi)| xi >= th && yi < thy && yi < 1.0).count();
            precision.push(tp as f64 / yb.len() as f64);
            fnr.push(fn_val as f64 / yb.len() as f64);
        }
    }
    (precision, fnr)
}

/// Aggregate bootstrapped curves: mean or percentile
pub fn aggregate_curves(curves: &[Vec<f64>], method: &str, percentile: f64, len: usize) -> Vec<f64> {
    let n = curves[0].len();
    match method {
        "mean" => (0..n).into_par_iter().map(|i| curves.iter().map(|v| v[i]).sum::<f64>() / len as f64).collect(),
        "percentile" => (0..n).into_par_iter().map(|i| {
            let mut col: Vec<f64> = curves.iter().map(|v| v[i]).collect();
            col.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let m = col.len();
            if m == 1 { return col[0]; }
            let h = (m-1) as f64 * percentile.clamp(0.0,1.0);
            let i0 = h.floor() as usize;
            let i1 = h.ceil() as usize;
            let frac = h - i0 as f64;
            (1.0-frac)*col[i0] + frac*col[i1]
        }).collect(),
        _ => panic!("Unknown aggregation method"),
    }
}

/// Bootstrap sampling
pub fn bootstrap_sample(x: &[f64], y: &[f64], n: usize, rng: &mut impl Rng) -> (Vec<f64>, Vec<f64>) {
    let mut xb = Vec::with_capacity(n);
    let mut yb = Vec::with_capacity(n);
    for _ in 0..n {
        let idx = rng.gen_range(0..n);
        xb.push(x[idx]);
        yb.push(y[idx]);
    }
    (xb, yb)
}

/// Linear interpolation (unsafe fast version)
pub fn linear_interpolate(x: &[f64], y: &[f64], xi: f64, n: usize) -> f64 {
    // trivial cases
    if xi < x[0] { eprintln!("too small {}", x[0]); return (y[0] * (xi - x[0]) + xi) / x[0]; } // lever rule between (0, 1) and first point
    if xi > x[n-1] { eprintln!("too big"); return f64::NAN; }
    if xi == x[n-1] { return y[n-1]; }
    
    let interp = Interp1d::new_sorted(x.to_vec(), y.to_vec());
    let yi = match interp {
        Ok(i) => i.interpolate(xi),
        Err(e) => {
            eprintln!("Interpolation failed: {:?}", e);
            return f64::NAN; 
        }
    };
    yi
}

/// Compute precision at target FNR robustly
pub fn precision_at_robust(
    x: &[f64],
    y: &[f64],
    quality_thresholds: &[f64],
    quantiles: &[f64],
    n_bootstrap: usize,
    method: &str,
    metric: &str,
    q: f64
) -> (Vec<f64>, Vec<f64>) {
    let mut all_precision = Vec::with_capacity(quality_thresholds.len());
    let mut all_thx = Vec::with_capacity(quality_thresholds.len());
    let n = quantiles.len();
    let conf = if metric.to_lowercase().contains("adpl") { 0.05 } else { 0.95 };

    for &thy in quality_thresholds {
        let (precision_curves, fnr_curves): (Vec<_>, Vec<_>) = (0..n_bootstrap)
            .into_par_iter()
            .map(|i| {
                let mut rng = StdRng::seed_from_u64(i as u64);
                let (xb, yb) = bootstrap_sample(x, y, n, &mut rng);
                compute_precision_fnr(&xb, &yb, thy, &quantiles)
            }).unzip();

        let precision_agg = aggregate_curves(&precision_curves, method, 1.0 - conf, n_bootstrap);
        let fnr_agg = aggregate_curves(&fnr_curves, method, conf, n_bootstrap);
        
        let (thx, precision_at_q) = { 
            let th_val = linear_interpolate(&fnr_agg, &quantiles, q, n);
            let prec_val = linear_interpolate(&fnr_agg, &precision_agg, q, n);
            (th_val, prec_val)
        };

        all_precision.push(precision_at_q);
        all_thx.push(thx);
    }
    (all_precision, all_thx)
}

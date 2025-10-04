use rayon::prelude::*;
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use interp1d::{Interp1d, error::InterpError};

/// Compute precision and false negative rate for a threshold
pub fn compute_tp_fn(
    xb: &[f64], 
    yb: &[f64], 
    thy: f64, 
    quantiles: &[f64]
) -> (Vec<f64>, Vec<f64>) {
    let mut tp = Vec::with_capacity(quantiles.len());
    let mut fnr = Vec::with_capacity(quantiles.len());

    for &th in quantiles {
        let n_y_high = yb.iter().filter(|&&v| v >= thy && v < 1.0).count();
        if n_y_high == 0 {
            tp.push(0.0);
            fnr.push(0.0);
        } else {
            let tp_val = xb.iter().zip(yb.iter()).filter(|(&xi, &yi)| xi >= th && yi >= thy && yi < 1.0).count();
            let fn_val = xb.iter().zip(yb.iter()).filter(|(&xi, &yi)| xi >= th && yi < thy && yi < 1.0).count();
            tp.push(tp_val as f64 / yb.len() as f64);
            fnr.push(fn_val as f64 / yb.len() as f64);
        }
    }
    (tp, fnr)
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
pub fn linear_interpolate(tp: &[f64], fnr: &[f64], quantiles: &[f64], xi: f64, n: usize) -> (f64, f64) {
    
    let mut p = Vec::with_capacity(n + 1);
    let mut f = Vec::with_capacity(n + 1);
    let mut q = Vec::with_capacity(n + 1); 
    // Prepend (0, 1) the (fn, tp) curve
    p.push(1.0); f.push(0.0);  
    // Then append the real data
    p.extend_from_slice(&tp); 
    f.extend_from_slice(&fnr); 
    q.extend_from_slice(&quantiles); 
    // append 1 to the quantiles
    q.push(1.0);
    
    let interp = Interp1d::new_sorted(f.clone(), p);
    let tp_val = match interp {
        Ok(i) => i.interpolate(xi),
        Err(e) => {
            eprintln!("Interpolation failed: {:?}", e);
            f64::NAN
        }
    };
    let interp = Interp1d::new_sorted(f, q);
    let th_val = match interp {
        Ok(i) => i.interpolate(xi),
        Err(e) => {
            eprintln!("Interpolation failed: {:?}", e);
            f64::NAN
        }
    };
    (th_val, tp_val)
}

/// Compute precision at target FNR robustly
pub fn tp_at_robust(
    x: &[f64],
    y: &[f64],
    quantiles: &[f64],
    quality_thresholds: &[f64],
    n_bootstrap: usize,
    method: &str,
    metric: &str,
    q: f64
) -> (Vec<f64>, Vec<f64>) {
    let nq = quantiles.len();
    let n = x.len();
    let conf = if metric.to_lowercase().contains("adpl") { 0.05 } else { 0.95 };
    let mut all_tp = Vec::with_capacity(quality_thresholds.len());
    let mut all_thx = Vec::with_capacity(quality_thresholds.len());

    for &thy in quality_thresholds {
        let (tp_curves, fn_curves): (Vec<_>, Vec<_>) = (0..n_bootstrap)
            .into_par_iter()
            .map(|i| {
                let mut rng = StdRng::seed_from_u64(i as u64);
                let (xb, yb) = bootstrap_sample(x, y, n, &mut rng);
                compute_tp_fn(&xb, &yb, thy, &quantiles)
            }).unzip();

        let tp_agg = aggregate_curves(&tp_curves, method, 1.0 - conf, n_bootstrap);
        let fn_agg = aggregate_curves(&fn_curves, method, conf, n_bootstrap);
        
        let (thx, tp_at_q) = linear_interpolate(&tp_agg, &fn_agg, &quantiles, q, nq);

        all_tp.push(tp_at_q);
        all_thx.push(thx);
    }
    (all_tp, all_thx)
}

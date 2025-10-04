use serde::{Deserialize, Serialize};
use plotly::{Plot, Scatter};
use plotly::common::{Mode, Visible};
use std::fs;
use std::collections::{HashSet, HashMap};

#[derive(Serialize, Deserialize)]
struct TraceData {
    method: String,
    dataset: String,
    fnr: f64,
    drop_level: f64,
    metric: String,
    aggregation: String,
    metric_aggregation: String,
    xs: Vec<f64>,
    ys: Vec<f64>,
}

// -------------------
// Utilities
// -------------------

fn add_traces(plot: &mut Plot, traces: &[TraceData]) {
    for trace in traces {
        let trace_name = format!(
            "{} | {} | FNR={:.2} | drop={:.1} | {} | {} | {}",
            trace.method, trace.dataset, trace.fnr, trace.drop_level,
            trace.metric, trace.aggregation, trace.metric_aggregation
        );
        let scatter = Scatter::new(trace.xs.clone(), trace.ys.clone())
            .mode(Mode::Lines)
            .name(&trace_name)
            .visible(Visible::LegendOnly); // initial visibility
        plot.add_trace(scatter);
    }
}

fn build_trace_map(traces: &[TraceData]) -> Vec<(usize, String)> {
    traces.iter().enumerate()
        .map(|(i, t)| {
            let key = format!("{}|{}|{}|{}|{}|{}|{}",
                              t.method, t.dataset, t.fnr, t.drop_level,
                              t.metric, t.aggregation, t.metric_aggregation);
            (i, key)
        }).collect()
}


fn unique_f64(mut vals: Vec<f64>) -> Vec<f64> {
    vals.sort_by(|a, b| a.partial_cmp(b).unwrap());
    vals.dedup();
    vals
} 

fn unique<T: Eq + std::hash::Hash + Clone>(vals: Vec<T>) -> Vec<T> {
    let mut seen = HashSet::new();
    vals.into_iter().filter(|x| seen.insert(x.clone())).collect()
}

fn make_dropdown(id: &str, options: &[String], defaults: &[String]) -> String {
    let opts: String = options.iter()
        .map(|o| {
            if defaults.contains(o) {
                format!(r#"<option value="{0}" selected>{0}</option>"#, o)
            } else {
                format!(r#"<option value="{0}">{0}</option>"#, o)
            }
        }).collect();
    format!(
        r#"<div style="display:flex; flex-direction:column; min-width:150px;">
            <label for="{0}" style="font-weight:bold; margin-bottom:4px;">{0}</label>
            <select id="{0}" multiple size="4" style="min-width:150px;">{1}</select>
        </div>"#,
        id, opts
    )
}

fn make_dropdown_f64(id: &str, options: &[f64], defaults: &[f64]) -> String {
    let opts: String = options.iter()
        .map(|o| {
            if defaults.contains(o) {
                format!(r#"<option value="{0}" selected>{0}</option>"#, o)
            } else {
                format!(r#"<option value="{0}">{0}</option>"#, o)
            }
        }).collect();
    format!(
        r#"<div style="display:flex; flex-direction:column; min-width:100px;">
            <label for="{0}" style="font-weight:bold; margin-bottom:4px;">{0}</label>
            <select id="{0}" multiple size="4" style="min-width:100px;">{1}</select>
        </div>"#,
        id, opts
    )
}

// -------------------
// Main
// -------------------

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load JSON
    let data_str = fs::read_to_string("Brats_last_final_traces.db")?;
    let traces_data: Vec<TraceData> = serde_json::from_str(&data_str)?;

    // Unique values for dropdowns
    let methods = unique(traces_data.iter().map(|t| t.method.clone()).collect());
    let datasets = unique(traces_data.iter().map(|t| t.dataset.clone()).collect());
    let drop_levels = unique_f64(traces_data.iter().map(|t| t.drop_level).collect());
    let fnrs = unique_f64(traces_data.iter().map(|t| t.fnr).collect()); 
    let metrics = unique(traces_data.iter().map(|t| t.metric.clone()).collect()); 
    let aggregations = unique(traces_data.iter().map(|t| t.aggregation.clone()).collect());
    let metric_aggs = unique(traces_data.iter().map(|t| t.metric_aggregation.clone()).collect());

    // Build trace map
    let trace_map = build_trace_map(&traces_data);
    let trace_map_dict: HashMap<String, usize> = trace_map.iter()
        .map(|(i, k)| (k.clone(), *i))
        .collect();
    let trace_map_json = serde_json::to_string(&trace_map_dict)?;

    // Default selections
    let fnr_defaults = vec![0.2];
    let drop_defaults = vec![0.1];
    let method_defaults = vec![methods[0].clone()];  // first method as default
    let dataset_defaults = vec![datasets[0].clone()]; 
    let metrics_defaults = vec![metrics[0].clone()]; 
    let aggregations_defaults = vec![aggregations[0].clone()]; 
    let metric_aggs_defaults = vec![metric_aggs[0].clone()]; // first dataset as default

    // Dropdown HTML
    let dropdown_html = format!(
        r#"<div id="dropdown-container" style="display:flex; gap:20px; flex-wrap:wrap; margin-bottom:20px;">
            {methods}
            {datasets}
            {fnrs}
            {drops}
            {metrics}
            {aggregations}
            {metric_aggs}
        </div>"#,
        methods = make_dropdown("method", &methods, &method_defaults),
        datasets = make_dropdown("dataset", &datasets, &dataset_defaults),
        fnrs = make_dropdown_f64("fnr", &fnrs, &fnr_defaults),
        drops = make_dropdown_f64("drop", &drop_levels, &drop_defaults),
        metrics = make_dropdown("metric", &metrics, &metrics_defaults),
        aggregations = make_dropdown("aggregation", &aggregations, &aggregations_defaults),
        metric_aggs = make_dropdown("metric_aggregation", &metric_aggs, &metric_aggs_defaults),
    );

    // -------------------
    // Generate HTML + JS
    // -------------------
    let mut plot = Plot::new();
    add_traces(&mut plot, &traces_data);
    
    // This already contains <div id="..."> + <script>...</script>
    let plot_html = plot.to_inline_html(None);
    
    // Build final HTML template
    let html = format!(r#"
    <!DOCTYPE html>
    <html>
    <head>
    <meta charset="utf-8">
    <title>Robust Curves</title>
    <script src="https://cdn.plot.ly/plotly-latest.min.js"></script>
    </head>
    <body>
    <div id="dropdown-container" style="display:flex; gap:20px; flex-wrap:wrap; margin-bottom:20px;">
        {dropdown_html}
    </div>
    
    {plot_html}
    
    <script>
    window.onload = function() {{
        const traceMap = {trace_map_json};
        let plot = document.getElementsByClassName('plotly-graph-div')[0];
    
        function updatePlot() {{
            const selectedMethods = Array.from(document.getElementById('method').selectedOptions).map(o => o.value);
            const selectedDatasets = Array.from(document.getElementById('dataset').selectedOptions).map(o => o.value);
            const selectedFnrs = Array.from(document.getElementById('fnr').selectedOptions).map(o => o.value); // compare as string
            const selectedDrops = Array.from(document.getElementById('drop').selectedOptions).map(o => o.value);
            const selectedMetrics = Array.from(document.getElementById('metric').selectedOptions).map(o => o.value);
            const selectedAggs = Array.from(document.getElementById('aggregation').selectedOptions).map(o => o.value);
            const selectedMetricAggs = Array.from(document.getElementById('metric_aggregation').selectedOptions).map(o => o.value);
        
            const visibility = Array(plot.data.length).fill(false);
            let count = 0;
        
            Object.entries(traceMap).forEach(([key, idx]) => {{
                if (count >= 20) return; // stop after 20 matches
                const [method, dataset, fnr, drop, metric, agg, metricAgg] = key.split('|');
                if (selectedMethods.includes(method) &&
                    selectedDatasets.includes(dataset) &&
                    selectedFnrs.includes(fnr) &&
                    selectedDrops.includes(drop) &&
                    selectedMetrics.includes(metric) &&
                    selectedAggs.includes(agg) &&
                    selectedMetricAggs.includes(metricAgg)) {{
                    visibility[idx] = true;
                    count++;
                }}
            }});
        
            Plotly.restyle(plot, {{visible: visibility}});
        }}
    
        const dropdowns = ['method','dataset','fnr','dropout','metric','aggregation','metric_aggregation'];
        dropdowns.forEach(id => document.getElementById(id).addEventListener('change', updatePlot));
    
        updatePlot(); // initial selection
    }};
    </script>
    </body>
    </html>
    "#,
        dropdown_html = dropdown_html,
        plot_html = plot_html,
        trace_map_json = trace_map_json
    );
    
    fs::write("robust_curves_full.html", html)?; 

    println!("✅ Wrote robust_curves_full.html");

    Ok(())
}


pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    main()?;
    Ok(())
}













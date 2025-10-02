use serde::{Deserialize, Serialize};
use plotly::{Plot, BoxPlot, Scatter};
use plotly::common::{BoxMean, Marker};
use std::fs;
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug)]
struct CorrRecord {
    class: String,
    correlation_type: String, // "pearson" or "spearman"
    correlation: f64,
    model: String,            // method/bin
    dataset: String,          // dataset
    x: String,                // metric
    y: String,                // raw metric name (unused here)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load JSON with all correlations
    let data_str = fs::read_to_string("correlations.json")?;
    let records: Vec<CorrRecord> = serde_json::from_str(&data_str)?;

    // Group by correlation type
    for corr_type in ["pearson", "spearman"] {
        let subset: Vec<&CorrRecord> = records
            .iter()
            .filter(|r| r.correlation_type == corr_type)
            .collect();

        let mut plot = Plot::new();

        // Group by (model, metric, dataset)
        let mut groups: HashMap<(String, String, String), Vec<f64>> = HashMap::new();
        for r in subset {
            groups
                .entry((r.model.clone(), r.x.clone(), r.dataset.clone()))
                .or_insert_with(Vec::new)
                .push(r.correlation);
        }

        for ((model, metric, dataset), vals) in groups {
            let label = format!("{}/{}/{}", model, metric, dataset);

            if vals.len() > 1 {
                // Boxplot if multiple values
                let trace = BoxPlot::new(vec![label.clone(); vals.len()], vals.clone())
                    .name(&label)
                    .box_mean(BoxMean::True)
                    .marker(Marker::new().opacity(0.6));
                plot.add_trace(trace);
            } else {
                // Scatter if only one value
                let trace = Scatter::new(vec![label.clone()], vec![vals[0]])
                    .name(&label)
                    .mode(plotly::common::Mode::Markers)
                    .marker(Marker::new().size(8).symbol(plotly::common::Symbol::Circle));
                plot.add_trace(trace);
            }
        }

        plot.set_layout(
            plotly::Layout::new()
                .boxmode(plotly::layout::BoxMode::Group) // group boxes side by side
                .y_axis(plotly::layout::Axis::new().title("Correlation").range(vec![0.0, 1.0]))
                .x_axis(plotly::layout::Axis::new().title("Method / Metric / Dataset"))
        );

        let html = plot.to_inline_html(None);
        fs::write(format!("correlations_{}.html", corr_type), html)?;
    }

    println!("? Wrote correlations_pearson.html and correlations_spearman.html");
    Ok(())
}

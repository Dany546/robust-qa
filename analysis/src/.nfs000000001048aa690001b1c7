// Cargo.toml dependencies (add these):
// [dependencies]
// serde = { version = "1.0", features = ["derive"] }
// serde_json = "1.0"
// plotly = "0.8"   # or the latest plotly crate version
// itertools = "0.10"

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use itertools::Itertools;
use plotly::{BoxPlot, Layout, Plot, Scatter};
use plotly::common::{BoxMean, Marker, Mode};
use plotly::layout::{Axis, Grid, GridPattern, Legend};

#[derive(Serialize, Deserialize, Debug)]
struct Record {
    seg_metric: String,   // e.g. "Dice"
    eval_metric: String,  // e.g. "Spearman", "ROC-AUC", "RQA"
    method: String,       // e.g. "UNet"
    uq_metric: String,    // e.g. "MC seg", "flip"
    dataset: String,      // e.g. "Brats"
    value: f64,           // numeric value to plot
}

/// Helper: a stable sorted list preserving natural order appearance in input, but deterministic
fn unique_ordered(values: Vec<String>) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for v in values {
        if seen.insert(v.clone()) {
            out.push(v);
        }
    }
    out
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load flat JSON
    let raw = fs::read_to_string("data_flat.json")?;
    let records: Vec<Record> = serde_json::from_str(&raw)?;

    if records.is_empty() {
        println!("No records found in data_flat.json");
        return Ok(());
    }

    // --- FIXED CONFIG ---
    // Evaluation metrics rows (in the order you requested):
    let eval_metrics = vec![
        "Spearman".to_string(),
        "ROC-AUC".to_string(),
        "RQA".to_string(),
    ];

    // Decide global dataset color mapping (keeps colors consistent across outputs)
    let mut datasets: Vec<String> = unique_ordered(records.iter().map(|r| r.dataset.clone()).collect());
    if datasets.is_empty() {
        println!("No datasets found.");
        return Ok(());
    }
    // Simple palette — extend if you have more datasets
    let palette = vec![
        "#1f77b4", "#ff7f0e", "#2ca02c", "#d62728",
        "#9467bd", "#8c564b", "#e377c2", "#7f7f7f",
    ];
    let mut dataset_color: HashMap<String, String> = HashMap::new();
    for (i, ds) in datasets.iter().enumerate() {
        dataset_color.insert(ds.clone(), palette[i % palette.len()].to_string());
    }

    // Methods and UQ metrics ordering: use appearance order in data (stable)
    let methods = unique_ordered(records.iter().map(|r| r.method.clone()).collect());
    let uq_metrics = unique_ordered(records.iter().map(|r| r.uq_metric.clone()).collect());
    let seg_metrics = unique_ordered(records.iter().map(|r| r.seg_metric.clone()).collect());

    // For each segmentation metric build a plot (rows = eval_metrics)
    for seg in seg_metrics {
        // Filter records for this seg metric
        let seg_records: Vec<&Record> = records.iter().filter(|r| r.seg_metric == seg).collect();

        // Build hierarchical categories: we want for every (uq_metric, method) pair an x-position.
        // We'll order by uq_metrics (outer) then methods (inner)
        let mut categories: Vec<(String, String)> = Vec::new(); // (uq_metric, method)
        for uq in uq_metrics.iter() {
            // include only methods that appear with this uq in the seg_records to avoid empty groups
            let methods_for_uq: Vec<String> = seg_records.iter()
                .filter(|r| &r.uq_metric == uq)
                .map(|r| r.method.clone())
                .unique()
                .collect();
            // If none, fallback to global methods to keep axis stable
            let chosen_methods = if methods_for_uq.is_empty() { methods.clone() } else { methods_for_uq };
            for m in chosen_methods {
                categories.push((uq.clone(), m));
            }
        }

        // For Layout grid: number of rows = eval_metrics.len(), columns = 1
        let nrows = eval_metrics.len();

        // We'll create one Plot and add traces. We'll place each eval_metric's traces on a separate subplot row.
        let mut plot = Plot::new();

        // For each row (eval metric) add all dataset traces. Each dataset will be one trace (consisting of all x positions where it has values).
        for (row_idx, eval_m) in eval_metrics.iter().enumerate() {
            // For each dataset gather values per (uq, method)
            for ds in datasets.iter() {
                // Build vectors for x (nested) and y values.
                // We will produce many repeated x entries for boxplots (Plotly expects x array aligned with y array)
                let mut x_nested: Vec<Vec<String>> = Vec::new(); // each element is vec![uq_metric, method]
                let mut y_vals: Vec<f64> = Vec::new();

                for (uq, method) in categories.iter() {
                    // collect values matching this cell: seg, eval_m, uq, method, ds
                    let cell_vals: Vec<f64> = seg_records.iter()
                        .filter(|r| &r.eval_metric == eval_m && &r.uq_metric == uq && &r.method == method && &r.dataset == ds)
                        .map(|r| r.value)
                        .collect();

                    if !cell_vals.is_empty() {
                        // push each value with the same x_nested label
                        for v in cell_vals {
                            x_nested.push(vec![uq.clone(), method.clone()]);
                            y_vals.push(v);
                        }
                    }
                }

                if y_vals.is_empty() {
                    // nothing to plot for this dataset in this eval row
                    continue;
                }

                // Choose trace type: BoxPlot if multiple values in at least one cell for this dataset+eval, else single scatter markers
                // To decide robustly: if any (uq,method) group produced >1 values, use boxplots (Plotly will group them).
                let use_box = {
                    // quickly test presence of any cell with >1 values for this dataset+eval_m
                    let mut any_multi = false;
                    for (uq, method) in categories.iter() {
                        let cnt = seg_records.iter()
                            .filter(|r| &r.eval_metric == eval_m && &r.uq_metric == uq && &r.method == method && &r.dataset == ds)
                            .count();
                        if cnt > 1 {
                            any_multi = true;
                            break;
                        }
                    }
                    any_multi
                };

                let ds_name = ds.clone();
                let color = dataset_color.get(ds).cloned().unwrap_or_else(|| "#333333".to_string());

                if use_box {
                    // BoxPlot takes Vec<Vec<String>> for hierarchical categories as x
                    let mut bp = BoxPlot::new(x_nested.clone(), y_vals.clone())
                        .name(&ds_name)
                        .box_mean(BoxMean::True)
                        .marker(Marker::new().color(color));
                    // assign this trace to a specific subplot row by setting xaxis/yaxis attributes:
                    // Plotly positions traces into subplot rows/cols by using layout grid and by setting "xaxis" and "yaxis" attributes.
                    // The plotly-rust crate exposes `x_axis` and `y_axis` methods on traces as `xaxis` and `yaxis` in some versions.
                    // We'll set xaxis/yaxis indexes via trace properties "xaxis" / "yaxis" (e.g., "x2", "y2").
                    let xaxis_name = format!("x{}", row_idx + 1); // x1, x2, ...
                    let yaxis_name = format!("y{}", row_idx + 1);
                    // In the plotly-rust crate older/newer versions the setter can differ; the `BoxPlot` struct exposes `x_axis`/`y_axis`.
                    // Use the 'set_x_axis' / 'set_y_axis' methods if your crate version uses those names.
                    // Fallback: use `bp = bp.x_axis(&xaxis_name).y_axis(&yaxis_name);`
                    bp = bp.x_axis(&xaxis_name).y_axis(&yaxis_name);
                    plot.add_trace(bp);
                } else {
                    // Scatter single points
                    let mut sc = Scatter::new(x_nested.clone(), y_vals.clone())
                        .name(&ds_name)
                        .mode(Mode::Markers)
                        .marker(Marker::new().color(color).size(8));
                    let xaxis_name = format!("x{}", row_idx + 1);
                    let yaxis_name = format!("y{}", row_idx + 1);
                    sc = sc.x_axis(&xaxis_name).y_axis(&yaxis_name);
                    plot.add_trace(sc);
                }
            } // end datasets loop
        } // end eval_metrics loop

        // --- Layout & Grid configuration ---
        // Create a grid with nrows rows and 1 column, pattern independent so each row has independent axes
        let mut grid = Grid::new();
        grid = grid.rows(nrows as i32).columns(1).pattern(GridPattern::Independent);

        // Build layout
        let mut layout = Layout::new();
        layout = layout.grid(grid)
            .legend(Legend::new().x(1.02).y(1.0).orientation(plotly::layout::LegendOrientation::V))
            .title(format!("Segmentation metric = {}", seg));

        // Create and configure yaxes and xaxes for each row
        for (row_idx, eval_m) in eval_metrics.iter().enumerate() {
            // y axis named y{index+1}
            let yname = format!("y{}", row_idx + 1);
            let yaxis = Axis::new()
                .title(eval_m)
                .range(vec![0.0, 1.0]); // correlations/metrics in [0,1] assumed; adjust if needed
            layout = layout.set(&yname, yaxis);

            // x axis named x{index+1}; we only show ticks on the bottom-most subplot to avoid duplicates,
            // but to preserve hierarchical appearance we'll show bottom ticks and add top annotations for UQ metric grouping.
            let xname = format!("x{}", row_idx + 1);
            // hide tick labels for intermediate rows except the bottom one:
            if row_idx < nrows - 1 {
                // hide tick labels for non-bottom rows
                let xaxis = Axis::new().show_tick_labels(false);
                layout = layout.set(&xname, xaxis);
            } else {
                // bottom row: set tick labels to the method names (lower-level)
                // Build tickvals and ticktext from categories
                let tickvals: Vec<String> = categories.iter().map(|(uq, m)| format!("{},{}", uq, m)).collect();
                // ticktext = methods (bottom)
                let ticktext: Vec<String> = categories.iter().map(|(_, m)| m.clone()).collect();
                let xaxis = Axis::new()
                    .tickmode(plotly::common::TickMode::Array)
                    .tickvals(tickvals.clone())
                    .ticktext(ticktext.clone())
                    .tickangle(0)
                    .title("Method / UQ metric");
                layout = layout.set(&xname, xaxis);
            }
        }

        // Add annotations above the bottom x-axis to represent UQ metric groups (top-level)
        // We'll compute contiguous ranges in the categories vector belonging to each uq metric and add centered annotations.
        let mut uq_group_indices: BTreeMap<String, (usize, usize)> = BTreeMap::new();
        for (idx, (uq, _method)) in categories.iter().enumerate() {
            uq_group_indices.entry(uq.clone())
                .and_modify(|e| e.1 = idx)
                .or_insert((idx, idx));
        }
        // Add annotation per UQ group. yref="paper" uses normalized coordinates; we place annotations slightly below the bottom of the lowest plot.
        // Note: annotation x uses the tickvalue label we set earlier: we used `"uq,method"` strings as tickvals for bottom axis.
        for (uq, (start_idx, end_idx)) in uq_group_indices {
            let mid_idx = (start_idx + end_idx) / 2;
            let tickval = format!("{},{}", categories[mid_idx].0, categories[mid_idx].1);
            // anchor in paper coordinates vertically slightly below 0 (since we share x-axes, choose yref='paper')
            layout = layout.annotation(
                plotly::layout::Annotation::new()
                    .x(tickval)
                    .xref(plotly::layout::Anchor::X)
                    .y(-0.08)
                    .yref(plotly::layout::Anchor::Paper)
                    .text(uq)
                    .showarrow(false)
                    .font(plotly::layout::Font::new().size(12))
            );
        }

        plot.set_layout(layout);

        // Write the html
        let filename = format!("plot_{}.html", seg.replace(|c: char| !c.is_alphanumeric(), "_"));
        let html = plot.to_inline_html(None);
        fs::write(&filename, html)?;
        println!("Wrote {}", filename);
    } // end seg_metrics loop

    println!("Done.");
    Ok(())
}

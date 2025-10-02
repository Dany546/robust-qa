pub mod config;
pub mod utils;

pub mod data {
    pub mod loader;
    pub mod xy_extractor;
    pub mod columnar;
}

pub mod metrics {
    pub mod precision;
}

pub mod plot {
    pub mod plot_html;
    pub mod plot_json;
}



use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TraceData {
    pub method: String,
    pub dataset: String,
    pub drop_level: f64,
    pub fnr: f64,
    pub metric: String,
    pub aggregation: String,
    pub metric_aggregation: String,
    pub xs: Vec<f64>,
    pub th: Vec<f64>,
    pub ys: Vec<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TraceFilter {
    pub method: Option<String>,
    pub dataset: Option<String>,
    pub metric: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TraceMeta {
    pub method: String,
    pub dataset: String,
    pub metric: String,
    pub aggregation: String,
    pub metric_aggregation: String,
}

// backend/src/api.rs

//! Role: Define REST endpoints for the frontend to consume.
//!
//! Endpoints:
//! - GET /traces/metadata ? return distinct methods, datasets, metrics, aggregations, drop_levels
//! - GET /traces ? return full traces, optionally filtered by query parameters

use axum::{
    extract::{Extension, Query},
    routing::get,
    Router, Json,
};
use crate::db;
use crate::models::{TraceData, TraceMeta};
use rusqlite::Connection;
use std::sync::Arc;
use std::collections::HashMap;

/// Create all backend routes and attach shared DB connection
pub fn create_routes(conn: Connection) -> Router {
    let shared_conn = Arc::new(conn);

    Router::new()
        .route("/traces", get(get_traces))
        .route("/traces/metadata", get(get_metadata))
        .layer(Extension(shared_conn))
}

/// GET /traces
/// Optional query parameters: method, dataset, metric, aggregation, drop_level
async fn get_traces(
    Extension(conn): Extension<Arc<Connection>>,
    Query(params): Query<HashMap<String, String>>,
) -> Json<Vec<TraceData>> {
    let traces = db::load_filtered_traces(
        &conn,
        params.get("method").map(|s| s.as_str()),
        params.get("dataset").map(|s| s.as_str()),
        params.get("metric").map(|s| s.as_str()),
        params.get("aggregation").map(|s| s.as_str()),
        params.get("metric_aggregation").map(|s| s.as_str()),
        params.get("drop_level").and_then(|s| s.parse::<f64>().ok()),
    ).unwrap_or_default();

    Json(traces)
}

/// GET /traces/metadata
/// Return all distinct values for dropdowns
async fn get_metadata(
    Extension(conn): Extension<Arc<Connection>>,
) -> Json<TraceMeta> {
    let meta = db::load_metadata(&conn).unwrap_or_default();
    Json(meta)
}

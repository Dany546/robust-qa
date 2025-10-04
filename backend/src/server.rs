use axum::{routing::get, Router, extract::Query};
use serde::Deserialize;
use std::sync::Arc;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;

#[tokio::main]
async fn main() {
    let pool = init_pool("db/main_db.sqlite");
    let shared_pool = Arc::new(pool);

    let app = Router::new()
        .route("/traces/metadata", get({
            let pool = shared_pool.clone();
            move || async move {
                let conn = pool.get().unwrap();
                let meta = load_metadata_sqlite("db/main_db.sqlite").unwrap();
                axum::Json(meta)
            }
        }))
        .route("/traces", get({
            let pool = shared_pool.clone();
            move |Query(params): Query<TraceFilter>| async move {
                let conn = pool.get().unwrap();
                let traces = load_traces_sqlite("db/main_db.sqlite").unwrap();
                axum::Json(traces) // TODO: filter by params
            }
        }));

    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

#[derive(Deserialize)]
struct TraceFilter {
    method: Option<String>,
    dataset: Option<String>,
    metric: Option<String>,
}

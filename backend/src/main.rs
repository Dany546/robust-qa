// backend/src/main.rs

use axum::{
    routing::get,
    Router,
};
use std::net::SocketAddr;
use std::sync::Arc;

use crate::db;
use crate::api;
use rusqlite::Connection;
use tower_http::trace::TraceLayer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize SQLite connection (read-only main DB)
    let conn = db::init_connection("db/main_db.sqlite", true)?;
    let conn = Arc::new(conn);

    // Build Axum app with API routes
    let app = api::create_routes((*conn).clone())
        .layer(TraceLayer::new_for_http());

    // Bind to 0.0.0.0:8080
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    println!("?? Backend listening on {}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

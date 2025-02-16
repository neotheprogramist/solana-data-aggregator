use std::io::Error;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use surrealdb::{Connection, Surreal};
use tokio::{
    net::TcpListener,
    sync::broadcast::{self},
};

#[derive(Debug, Clone)]
struct ServerState<C: Connection + Clone> {
    db: Surreal<C>,
}

pub async fn run<C: Connection + Clone>(
    listener: TcpListener,
    db: Surreal<C>,
    mut shutdown: broadcast::Receiver<()>,
) -> Result<(), Error> {
    let state = ServerState { db };

    let app = Router::new()
        .route("/transactions", get(handle))
        .with_state(state);

    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            shutdown.recv().await.unwrap();
            tracing::info!("server shutting down...");
        })
        .await?;

    Ok(())
}

#[derive(Debug, Deserialize)]
struct Q {
    id: Option<String>,
    day: Option<NaiveDate>,
}
async fn handle<C: Connection + Clone>(
    Query(q): Query<Q>,
    State(state): State<ServerState<C>>,
) -> Result<impl IntoResponse, StatusCode> {
    #[derive(Debug, Deserialize, Serialize)]
    struct Transaction {
        signature: String,
        slot: u64,
        block_hash: String,
        timestamp: i64,
    }

    // Query by signature if provided
    if let Some(id) = q.id {
        let mut result = state
            .db
            .query("SELECT signature, slot, block_hash, timestamp FROM type::table($table) WHERE signature = type::string($signature)")
            .bind(("table", "transactions"))
            .bind(("signature", id))
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let transaction: Vec<Transaction> = result.take(0).unwrap();
        return Ok(Json(transaction));
    }

    // Query by day if provided
    if let Some(day) = q.day {
        let start_timestamp = day.and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp();
        let end_timestamp = day.and_hms_opt(23, 59, 59).unwrap().and_utc().timestamp();

        let mut result = state
            .db
            .query("SELECT signature, slot, block_hash, timestamp FROM type::table($table) WHERE timestamp >= $start AND timestamp <= $end")
            .bind(("table", "transactions"))
            .bind(("start", start_timestamp))
            .bind(("end", end_timestamp))
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let transactions: Vec<Transaction> = result.take(0).unwrap();
        return Ok(Json(transactions));
    }

    Err(StatusCode::NOT_FOUND)
}

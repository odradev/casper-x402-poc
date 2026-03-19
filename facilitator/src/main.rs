mod casper;
mod routes;
mod types;

use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use casper::settler::CasperSettler;

#[derive(Clone)]
pub struct AppState {
    pub settler: Arc<CasperSettler>,
}

#[tokio::main]
async fn main() {
    let _ = dotenv::dotenv();

    let settler = CasperSettler::from_env().expect("Failed to initialize CasperSettler");
    let state = AppState {
        settler: Arc::new(settler),
    };

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3001);

    let app = Router::new()
        .route("/verify", post(routes::verify::handle_verify))
        .route("/settle", post(routes::settle::handle_settle))
        .route("/supported", get(routes::supported::handle_settle))
        .with_state(state);

    let addr = format!("0.0.0.0:{}", port);
    println!("[facilitator] Listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind");
    axum::serve(listener, app).await.expect("Server error");
}

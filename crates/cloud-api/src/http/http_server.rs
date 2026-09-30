use axum::{Router, routing::get};

async fn health() -> &'static str {
    "BuliCloud is running!"
}

pub async fn start_http_server() -> std::io::Result<()> {
    let app = Router::new().route("/health", get(health));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;

    axum::serve(listener, app).await.map_err(std::io::Error::other)
}

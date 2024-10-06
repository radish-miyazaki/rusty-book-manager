use std::net::{Ipv4Addr, SocketAddr};

use anyhow::{Ok, Result};
use axum::{http::StatusCode, routing::get, Router};
use tokio::net::TcpListener;

async fn health_check() -> StatusCode {
    StatusCode::OK
}

#[tokio::main]
async fn main() -> Result<()> {
    let app = Router::new().route("/health", get(health_check));
    let addr = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 8080);
    let listner = TcpListener::bind(addr).await?;

    println!("Listening on {}", addr);

    Ok(axum::serve(listner, app).await?)
}

#[cfg(test)]
mod tests {
    use axum::http::StatusCode;

    use crate::health_check;

    #[tokio::test]
    async fn health_check_works() {
        let status_code = health_check().await;
        assert_eq!(status_code, StatusCode::OK)
    }
}

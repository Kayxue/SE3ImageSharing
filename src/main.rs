use std::{env, net::SocketAddr};

use axum::{Router, extract::DefaultBodyLimit};
use dotenv::dotenv;
use tokio::{fs, main, net::TcpListener};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod middleware;
mod routes;

use routes::image::image_router;

#[main]
async fn main() {
    dotenv().ok();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{}=debug", env!("CARGO_CRATE_NAME")).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let api_key = env::var("API_KEY").expect("API_KEY not set");
    let image_dir = env::var("IMAGE_DIR").expect("IMAGE_DIR not set");

    fs::create_dir_all(&image_dir).await.unwrap();

    let app = Router::new()
        .merge(image_router(api_key, image_dir))
        .layer(DefaultBodyLimit::max(5 * 1024 * 1024)); // 5 MB

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    tracing::debug!("listening on {addr}");

    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

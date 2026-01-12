use axum::{routing::get, Router};
use tower_http::services::ServeDir;

mod context;
mod middleware;
mod pages;

use middleware::{SecurityHeadersLayer, UrlCanonicalizationLayer};

#[tokio::main]
async fn main() {
    // Load .env file if present (for local development)
    let _ = dotenvy::dotenv();

    let listen_addr = std::env::var("LISTEN_ADDR").unwrap_or_else(|_| "127.0.0.1:3000".to_string());

    // Build router with middleware
    let app = Router::new()
        // Pages - these are the canonical URLs that get served
        .route("/", get(pages::home::get))
        .route("/checksums.htm", get(pages::checksums::get).post(pages::checksums::post))
        .route("/about.htm", get(pages::about::get))
        // Static files at original URLs (matching PHP site structure)
        .nest_service("/images", ServeDir::new("static/images"))
        .nest_service("/js", ServeDir::new("static/js"))
        // CSS files at root (like /main.css)
        .nest_service("/main.css", ServeDir::new("static/main.css"))
        .nest_service("/mainmenu.css", ServeDir::new("static/mainmenu.css"))
        .nest_service("/vimhl.css", ServeDir::new("static/vimhl.css"))
        .nest_service("/print.css", ServeDir::new("static/print.css"))
        // 404 fallback for unmatched routes
        .fallback(pages::not_found::handler)
        // Apply middleware layers (outermost first)
        .layer(SecurityHeadersLayer)
        .layer(UrlCanonicalizationLayer);

    println!("Listening on http://{}", listen_addr);

    let listener = tokio::net::TcpListener::bind(&listen_addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

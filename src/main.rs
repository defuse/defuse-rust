use axum::{routing::get, Router};
use tower_http::services::ServeDir;

mod context;
mod pages;

#[tokio::main]
async fn main() {
    // Load .env file if present (for local development)
    let _ = dotenvy::dotenv();

    let listen_addr = std::env::var("LISTEN_ADDR").unwrap_or_else(|_| "127.0.0.1:3000".to_string());

    // Build router
    let app = Router::new()
        // Pages
        .route("/", get(pages::home::get))
        .route("/checksums", get(pages::checksums::get).post(pages::checksums::post))
        .route("/checksums.htm", get(pages::checksums::get).post(pages::checksums::post))
        .route("/about", get(pages::about::get))
        .route("/about.htm", get(pages::about::get))
        // Static files at original URLs (matching PHP site structure)
        .nest_service("/images", ServeDir::new("static/images"))
        .nest_service("/js", ServeDir::new("static/js"))
        // CSS files at root (like /main.css)
        .nest_service("/main.css", ServeDir::new("static/main.css"))
        .nest_service("/mainmenu.css", ServeDir::new("static/mainmenu.css"))
        .nest_service("/vimhl.css", ServeDir::new("static/vimhl.css"))
        .nest_service("/print.css", ServeDir::new("static/print.css"));

    println!("Listening on http://{}", listen_addr);

    let listener = tokio::net::TcpListener::bind(&listen_addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

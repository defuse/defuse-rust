use axum::{middleware as axum_middleware, routing::get, Router};
use tower_http::{catch_panic::CatchPanicLayer, services::ServeDir};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod context;
mod db;
mod middleware;
mod pages;
mod state;

use db::PhpCountService;
use middleware::{hit_counter_middleware, SecurityHeadersLayer, UrlCanonicalizationLayer};
use state::AppState;

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "defuse=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load .env file if present (for local development)
    let _ = dotenvy::dotenv();

    let listen_addr = std::env::var("LISTEN_ADDR").unwrap_or_else(|_| "127.0.0.1:3000".to_string());

    // Connect to databases (fail fast if unavailable)
    let phpcount_url = std::env::var("PHPCOUNT_DATABASE_URL")
        .expect("PHPCOUNT_DATABASE_URL must be set");

    tracing::info!("Connecting to PHPCount database...");
    let phpcount = PhpCountService::connect(&phpcount_url)
        .await
        .expect("Failed to connect to PHPCount database");
    tracing::info!("PHPCount database connected");

    // Create application state
    let state = AppState::new(phpcount);

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
        // CatchPanicLayer: ensures a panic in any handler returns 500, not crash
        .layer(CatchPanicLayer::new())
        .layer(SecurityHeadersLayer)
        // Hit counter middleware - records page hits, stores counts in extensions
        .layer(axum_middleware::from_fn_with_state(
            state.clone(),
            hit_counter_middleware,
        ))
        .layer(UrlCanonicalizationLayer)
        .with_state(state);

    tracing::info!("Listening on http://{}", listen_addr);

    let listener = tokio::net::TcpListener::bind(&listen_addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

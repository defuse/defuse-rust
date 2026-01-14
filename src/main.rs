use axum::{middleware as axum_middleware, routing::post, Router};
use tower_http::{catch_panic::CatchPanicLayer, services::ServeDir};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod app_state;
mod context;
mod dispatcher;
mod handler;
mod libs;
mod middleware;
mod pages;
mod registry;
mod upvote;

use app_state::AppState;
use libs::{PhpCountService, UpvoteService};
use middleware::{upvote_post_middleware, SecurityHeadersLayer, UrlCanonicalizationLayer};

#[tokio::main]
async fn main() {
    // Initialize logging
    // Default to info level; set RUST_LOG=defuse=debug for verbose output
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "defuse=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load .env file if present (for local development)
    let _ = dotenvy::dotenv();

    let listen_addr =
        std::env::var("LISTEN_ADDR").unwrap_or_else(|_| "127.0.0.1:3000".to_string());

    // Connect to databases (fail fast if unavailable)
    let phpcount_url =
        std::env::var("PHPCOUNT_DATABASE_URL").expect("PHPCOUNT_DATABASE_URL must be set");
    let upvotes_url =
        std::env::var("UPVOTES_DATABASE_URL").expect("UPVOTES_DATABASE_URL must be set");

    tracing::info!("Connecting to PHPCount database...");
    let phpcount = PhpCountService::connect(&phpcount_url)
        .await
        .expect("Failed to connect to PHPCount database");
    tracing::info!("PHPCount database connected");

    tracing::info!("Connecting to Upvotes database...");
    let upvotes = UpvoteService::connect(&upvotes_url)
        .await
        .expect("Failed to connect to Upvotes database");
    tracing::info!("Upvotes database connected");

    // Create application state
    let state = AppState::new(phpcount, upvotes);

    // Build router with middleware
    let app = Router::new()
        // API endpoints (not pages - handled explicitly)
        .route("/upvote", post(upvote::post))
        // Static files at original URLs (matching PHP site structure)
        .nest_service("/images", ServeDir::new("static/images"))
        .nest_service("/js", ServeDir::new("static/js"))
        // CSS files at root (like /main.css)
        .nest_service("/main.css", ServeDir::new("static/main.css"))
        .nest_service("/mainmenu.css", ServeDir::new("static/mainmenu.css"))
        .nest_service("/vimhl.css", ServeDir::new("static/vimhl.css"))
        .nest_service("/print.css", ServeDir::new("static/print.css"))
        // Verification files
        .nest_service(
            "/googlec56659c80ebb2d30.html",
            ServeDir::new("static/googlec56659c80ebb2d30.html"),
        )
        .nest_service(
            "/have-i-been-pwned-verification.txt",
            ServeDir::new("static/have-i-been-pwned-verification.txt"),
        )
        .nest_service(
            "/b06c1611fa2dac4cc177a7a63c70902c453cdb92.html",
            ServeDir::new("static/b06c1611fa2dac4cc177a7a63c70902c453cdb92.html"),
        )
        // Favicon
        .nest_service("/favicon.ico", ServeDir::new("static/favicon.ico"))
        // robots.txt
        .nest_service("/robots.txt", ServeDir::new("static/robots.txt"))
        // All pages handled by dispatcher (registry-driven routing)
        .fallback(dispatcher::handle)
        // Apply middleware layers (outermost first)
        // CatchPanicLayer: ensures a panic in any handler returns 500, not crash
        .layer(CatchPanicLayer::new())
        .layer(SecurityHeadersLayer)
        // Upvote POST fallback - handles votes when JS is disabled, redirects after
        .layer(axum_middleware::from_fn_with_state(
            state.clone(),
            upvote_post_middleware,
        ))
        .layer(UrlCanonicalizationLayer)
        .with_state(state);

    tracing::info!("Listening on http://{}", listen_addr);

    let listener = tokio::net::TcpListener::bind(&listen_addr).await.unwrap();
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await
    .unwrap();
}

use axum::{middleware as axum_middleware, routing::{any, get, get_service, post}, Router};
use tower_http::{catch_panic::CatchPanicLayer, services::ServeDir};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod app_state;
mod context;
mod dispatcher;
mod handler;
mod libs;
mod middleware;
mod pages;
mod prelude;
mod registry;
mod special_endpoints;
mod storage_routes;
mod upvote;

use app_state::AppState;
use axum::{
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};
use libs::{PastebinService, PhpCountService, UpvoteService};
use middleware::{blocking_middleware, upvote_post_middleware, EtagLayer, SecurityHeadersLayer, UrlCanonicalizationLayer};

/// Create a 301 Moved Permanently redirect response
fn redirect_301(location: &'static str) -> Response {
    (
        StatusCode::MOVED_PERMANENTLY,
        [(header::LOCATION, location)],
    )
        .into_response()
}

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

    // Required configuration (fail fast if not set)
    let storage_path = std::path::PathBuf::from(
        std::env::var("STORAGE_PATH").expect("STORAGE_PATH must be set"),
    );
    let phpcount_url =
        std::env::var("PHPCOUNT_DATABASE_URL").expect("PHPCOUNT_DATABASE_URL must be set");
    let upvotes_url =
        std::env::var("UPVOTES_DATABASE_URL").expect("UPVOTES_DATABASE_URL must be set");
    let pastebin_url =
        std::env::var("PASTEBIN_DATABASE_URL").expect("PASTEBIN_DATABASE_URL must be set");

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

    tracing::info!("Connecting to Pastebin database...");
    let pastebin = PastebinService::connect(&pastebin_url)
        .await
        .expect("Failed to connect to Pastebin database");
    tracing::info!("Pastebin database connected");

    // Create application state
    let state = AppState::new(phpcount, upvotes, pastebin);

    // Build router with middleware
    let app = Router::new()
        // API endpoints (not pages - handled explicitly)
        .route("/upvote.php", post(upvote::post))
        // Time capsule archive download
        .route(
            "/timecapsule/quantum-computer-time-capsule-download.php",
            get(pages::services::quantum_computer_time_capsule::download_archive),
        )
        // Special utility endpoints
        .route("/ip.php", get(special_endpoints::ip_php))
        .route("/ip-insecure.php", get(special_endpoints::ip_insecure_php))
        .route("/getmyip.php", get(special_endpoints::getmyip_php))
        .route("/s.php", get(special_endpoints::shout_php))
        // Pastebin routes
        .route("/bin/add.php", post(pages::services::pastebin_add::handler))
        .route("/bin/", get(|| async { redirect_301("/pastebin.htm") }))
        .route("/bin", get(|| async { redirect_301("/pastebin.htm") }))
        .route("/b/", get(pages::services::pastebin_view::bin_index_handler))
        .route("/b", get(|| async { redirect_301("/pastebin.htm") }))
        .route("/b/:key", get(pages::services::pastebin_view::handler))
        // Storage directories (files, files2, mirrors, upload from STORAGE_PATH)
        .merge(storage_routes::storage_router(&storage_path, state.clone()))
        // Fallback: static files for GET/HEAD, dispatcher for POST and when files not found
        .fallback_service(
            get_service(
                ServeDir::new("static")
                    .fallback(any(dispatcher::handle).with_state(state.clone())),
            )
            .post(dispatcher::handle)
            .with_state(state.clone()),
        )
        // Apply middleware layers (outermost first)
        // CatchPanicLayer: ensures a panic in any handler returns 500, not crash
        .layer(CatchPanicLayer::new())
        // EtagLayer: adds ETag headers to static files (must be after SecurityHeaders
        // so it can see Cache-Control: no-store and skip ETags for sensitive pages)
        .layer(EtagLayer)
        .layer(SecurityHeadersLayer)
        // Upvote POST fallback - handles votes when JS is disabled, redirects after
        .layer(axum_middleware::from_fn_with_state(
            state.clone(),
            upvote_post_middleware,
        ))
        .layer(UrlCanonicalizationLayer)
        // BlockingMiddleware: runs handlers on blocking thread pool for OS preemption
        // This is innermost so actual request handling gets preemptive scheduling
        .layer(axum_middleware::from_fn(blocking_middleware))
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

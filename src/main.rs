//! defuse.ca - Port of my defuse.ca website from the original PHP code to Rust.

use axum::{middleware as axum_middleware, routing::{any, get, get_service, post}, Router};
use tower_http::{catch_panic::CatchPanicLayer, services::ServeDir};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod app_state;
mod context;
mod registered_page_handler;
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
use libs::{PhpCountService, UpvoteService};
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

    // Note this codebase only supports HTTP, not HTTPS. Caddy must be configured as a reverse proxy for HTTPS.
    let listen_addr =
        std::env::var("LISTEN_ADDR").unwrap_or_else(|_| "127.0.0.1:3000".to_string());

    // Required configuration (fail fast if not set)
    let storage_path = std::path::PathBuf::from(
        std::env::var("STORAGE_PATH").expect("STORAGE_PATH must be set"),
    );

    // Do the database connections at startup (not on the fly) so that misconfigurations are detected early.

    // phpcount database
    let phpcount_url = std::env::var("PHPCOUNT_DATABASE_URL").expect("PHPCOUNT_DATABASE_URL must be set");
    tracing::info!("Connecting to PHPCount database...");
    let phpcount = PhpCountService::connect(&phpcount_url).await.expect("Failed to connect to PHPCount database");
    tracing::info!("PHPCount database connected");

    // upvote database
    let upvotes_url = std::env::var("UPVOTES_DATABASE_URL").expect("UPVOTES_DATABASE_URL must be set");
    tracing::info!("Connecting to Upvotes database...");
    let upvotes = UpvoteService::connect(&upvotes_url).await.expect("Failed to connect to Upvotes database");
    tracing::info!("Upvotes database connected");

    // pastebin database
    tracing::info!("Connecting to Pastebin database...");
    libs::pastebin::ensure_db_connection_works().await.expect("Failed to connect to Pastebin database");
    tracing::info!("Pastebin database connected");

    // trent database
    tracing::info!("Connecting to TRENT database...");
    libs::trent::ensure_db_connection_works().await.expect("Failed to connect to TRENT database");
    tracing::info!("TRENT database connected");

    // time capsule database
    tracing::info!("Connecting to Time Capsule database...");
    libs::timecapsule::ensure_db_connection_works().await.expect("Failed to connect to Time Capsule database");
    tracing::info!("Time Capsule database connected");

    // Create application state
    let state = AppState::new(phpcount, upvotes);

    // The main router handling all requests, with middleware.
    let app = Router::new()
        // Upvote submission forms (individual pages also must handle upvote POSTs)
        .route("/upvote.php", post(upvote::post))
        // Time capsule download
        .route(
            "/timecapsule/quantum-computer-time-capsule-download.php",
            get(pages::services::quantum_computer_time_capsule::download_archive),
        )
        // One-off PHP scripts I had in the root
        .route("/ip.php", get(special_endpoints::ip_php))
        .route("/ip-insecure.php", get(special_endpoints::ip_insecure_php))
        .route("/getmyip.php", get(special_endpoints::getmyip_php))
        .route("/s.php", get(special_endpoints::shout_php))
        // Pastebin routes and redirects
        .route("/bin/add.php", post(pages::services::pastebin_add::handler))
        .route("/bin/", get(|| async { redirect_301("/pastebin.htm") }))
        .route("/bin", get(|| async { redirect_301("/pastebin.htm") }))
        .route("/b/", get(pages::services::pastebin_view::bin_index_handler))
        .route("/b", get(|| async { redirect_301("/pastebin.htm") }))
        .route("/b/:key", get(pages::services::pastebin_view::handler))

        // Storage directories (files, files2, mirrors, upload)
        // This does NOT serve /storage itself (that contains credentials!)
        .merge(storage_routes::storage_router(&storage_path, state.clone()))

        // In the PHP version, css/js files were just in the root. We've moved 
        // them into static/, so when a request comes in for "/main.css", we
        // want to serve the file "static/main.css".
        // 
        // We can also put things like robots.txt and site verification files, or
        // straight up html files like longcat.html in static/.
        .fallback_service(
            get_service(
                ServeDir::new("static")
                    // This fallback is the main handler for registered pages
                    .fallback(any(registered_page_handler::handle).with_state(state.clone())),
            )
            // Handler for POST requests to registered pages.
            .post(registered_page_handler::handle)
            .with_state(state.clone()),
        )

        // The way middleware works is that the outermost layer sees the request
        // first, passes on to the next layer, eventually arriving at the
        // innermost middleware, then the stack unwinds.

        // Makes sure any panic results in a 500 error rather than a crash.
        .layer(CatchPanicLayer::new())

        // Adds ETag headers to static files 
        // Must wrap SecurityHeaders so it can see Cache-Control: no-store and skip ETags for sensitive pages.
        .layer(EtagLayer)
        // Handles various things like HSTS headers, not caching pastebin posts, etc.
        .layer(SecurityHeadersLayer)
        // Upvote POST fallback - handles votes when JS is disabled, redirects after
        .layer(axum_middleware::from_fn_with_state(
            state.clone(),
            upvote_post_middleware,
        ))
        // Handles things like:
        //  - Redirect to HTTPS
        //  - /abouT -> /about.htm
        //  - etc.
        .layer(UrlCanonicalizationLayer)
        // BlockingMiddleware: runs handlers on blocking thread pool for OS preemption
        // This is innermost so actual request handling gets preemptive scheduling

        // Runs handlers on Tokio's blocking thread pool so the OS can preempt
        // CPU-bound work (e.g. hashing large files on the checksums page).
        // Without this, a long computation would block the async runtime and
        // starve all other requests.
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

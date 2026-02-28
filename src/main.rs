//! defuse.ca - Port of my defuse.ca website from the original PHP code to Rust.
//! Copyright (C) 2026  Taylor Hornby
//!
//! This program is free software: you can redistribute it and/or modify
//! it under the terms of the GNU Affero General Public License as
//! published by the Free Software Foundation, either version 3 of the
//! License, or (at your option) any later version.
//!
//! This program is distributed in the hope that it will be useful,
//! but WITHOUT ANY WARRANTY; without even the implied warranty of
//! MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
//! GNU Affero General Public License for more details.
//!
//! You should have received a copy of the GNU Affero General Public License
//! along with this program.  If not, see <https://www.gnu.org/licenses/>.

use axum::{extract::DefaultBodyLimit, middleware as axum_middleware, routing::{any, get, get_service, post}, Router};
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
use middleware::{blocking_middleware, upvote_post_middleware, SecurityHeadersLayer, UrlCanonicalizationLayer};

/// Create a 301 Moved Permanently redirect response
fn redirect_301(location: &'static str) -> Response {
    (
        StatusCode::MOVED_PERMANENTLY,
        [(header::LOCATION, location)],
    )
        .into_response()
}

fn main() {
    // Build runtime with a higher blocking thread pool limit (default is 512).
    // Every request runs on a blocking thread (via blocking_middleware), so this
    // effectively limits max concurrent requests.
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .max_blocking_threads(4096)
        .build()
        .expect("failed to build Tokio runtime");

    runtime.block_on(async_main());
}

async fn async_main() {
    // Limit pest parser recursion depth to prevent stack overflow crashes.
    // Stack overflows bypass CatchPanicLayer (they abort, not unwind), so a
    // deeply-nested expression like (1+(1+(1+...))) would kill the process.
    // Each nesting level uses ~150 rule calls (many precedence levels in the
    // grammar), so 100,000 allows ~666 levels of nesting.
    pest::set_call_limit(std::num::NonZeroUsize::new(100_000));

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
    // Actually, don't, because that's confusing if a developer is trying to set
    // env vars manually but this is overwriting them.
    // let _ = dotenvy::dotenv();

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

    // Sync all registered pages to upvote database (ensures categories/metadata are current)
    upvotes.sync_all_pages().await.expect("Failed to sync pages to upvote database");
    tracing::info!("Upvote pages synced");

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

    // Validate required env vars that aren't checked by the above connections
    std::env::var("RECAPTCHA_SECRET_KEY").expect("RECAPTCHA_SECRET_KEY must be set");

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
        .route("/bin/add.php", post(pages::services::pastebin_add::handler)
            .layer(DefaultBodyLimit::max(100 * 1024 * 1024))) // 100 MB so the handler's 50 MB check returns a useful error
        .route("/bin/", get(|| async { redirect_301("/pastebin.htm") }))
        .route("/bin", get(|| async { redirect_301("/pastebin.htm") }))
        .route("/b/", get(|| async { redirect_301("/pastebin.htm") }))
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
        // innermost middleware, then the stack unwinds. Here, the INNERMOST
        // layers come first.

        // NOTE: Caddy is responsible for rejecting POSTs with too-big bodies (100MB).

        // BlockingMiddleware: runs handlers on blocking thread pool for OS preemption
        // This is innermost so actual request handling gets preemptive scheduling
        //
        // Runs handlers on Tokio's blocking thread pool so the OS can preempt
        // CPU-bound work (e.g. hashing large files on the checksums page).
        // Without this, a long computation would block the async runtime and
        // starve all other requests.
        .layer(axum_middleware::from_fn(blocking_middleware))
        .with_state(state.clone())

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

        // Handles various things like HSTS headers, not caching pastebin posts, etc.
        .layer(SecurityHeadersLayer)

        // Makes sure any panic results in a 500 error rather than a crash.
        .layer(CatchPanicLayer::new());


    tracing::info!("Listening on http://{}", listen_addr);

    let listener = tokio::net::TcpListener::bind(&listen_addr).await.unwrap();
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(async {
        tokio::signal::ctrl_c().await.expect("failed to listen for Ctrl+C");
        eprintln!("Shutting down gracefully (Ctrl+C again to force quit)...");
        // Reset SIGINT to default OS behavior so the next Ctrl+C kills immediately
        unsafe { libc::signal(libc::SIGINT, libc::SIG_DFL); }
    })
    .await
    .unwrap();
}

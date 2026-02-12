//! RegelRecht Upload Portal
//!
//! A web application for teams to share internal rule sets and policy documents
//! for the RegelRecht Proof of Concept.
//!
//! ## Features
//!
//! - **Applicant Portal**: Submit documents with classification
//! - **Admin Portal**: Manage submissions, schedule meetings
//! - **Calendar Integration**: Book meeting slots for document review

mod config;
mod db;
mod handlers;
mod models;
mod validation;

use axum::{
    routing::{delete, get, post, put},
    Router,
};
use handlers::AppState;
use std::path::PathBuf;
use tokio::fs;
use tower_http::{
    cors::{Any, CorsLayer},
    limit::RequestBodyLimitLayer,
    services::ServeDir,
    trace::TraceLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "regelrecht_upload=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load configuration
    let config = config::Config::from_env()?;
    tracing::info!("Starting RegelRecht Upload Portal");
    tracing::info!("Environment: {:?}", config.environment);

    // Create database pool
    tracing::info!("Connecting to database...");
    let pool = db::create_pool(&config.database_url).await?;
    tracing::info!("Database connected");

    // Run migrations
    tracing::info!("Running database migrations...");
    db::run_migrations(&pool).await?;

    // Ensure upload directory exists
    let upload_dir = PathBuf::from(&config.upload_dir);
    fs::create_dir_all(&upload_dir).await?;
    tracing::info!("Upload directory: {:?}", upload_dir);

    // Create application state
    let state = AppState {
        pool: pool.clone(),
        upload_dir,
        max_upload_size: config.max_upload_size,
    };

    // Build CORS layer
    let cors = if config.is_production() {
        CorsLayer::new()
            .allow_origin(
                config
                    .cors_origins
                    .iter()
                    .filter_map(|o| o.parse().ok())
                    .collect::<Vec<_>>(),
            )
            .allow_methods(Any)
            .allow_headers(Any)
            .allow_credentials(true)
    } else {
        CorsLayer::permissive()
    };

    // Build API routes
    let api_routes = Router::new()
        // Applicant submission endpoints
        .route("/submissions", post(handlers::create_submission))
        .route("/submissions/:slug", get(handlers::get_submission))
        .route("/submissions/:slug", put(handlers::update_submission))
        .route(
            "/submissions/:slug/submit",
            post(handlers::submit_submission),
        )
        .route(
            "/submissions/:slug/documents",
            post(handlers::upload_document),
        )
        .route(
            "/submissions/:slug/formal-law",
            post(handlers::add_formal_law),
        )
        .route(
            "/submissions/:slug/documents/:doc_id",
            delete(handlers::delete_document),
        )
        // Calendar endpoints (public)
        .route("/calendar/available", get(handlers::get_available_slots))
        .route("/submissions/:slug/book-slot", post(handlers::book_slot))
        .route(
            "/submissions/:slug/cancel-booking",
            post(handlers::cancel_booking),
        )
        // FAQ
        .route("/faq", get(handlers::get_faq))
        // Admin authentication
        .route("/admin/login", post(handlers::admin_login))
        .route("/admin/logout", post(handlers::admin_logout))
        .route("/admin/me", get(handlers::get_current_admin))
        // Admin submission management
        .route("/admin/submissions", get(handlers::list_submissions))
        .route(
            "/admin/submissions/:id",
            get(handlers::get_submission_admin),
        )
        .route(
            "/admin/submissions/:id/status",
            put(handlers::update_submission_status),
        )
        .route(
            "/admin/submissions/:id/forward",
            post(handlers::forward_submission),
        )
        .route("/admin/dashboard", get(handlers::get_dashboard_stats))
        // Admin calendar management
        .route("/admin/calendar/slots", get(handlers::list_slots_admin))
        .route("/admin/calendar/slots", post(handlers::create_slots))
        .route(
            "/admin/calendar/slots/:slot_id",
            delete(handlers::delete_slot),
        );

    // Build main router
    let app = Router::new()
        .nest("/api", api_routes)
        .nest_service("/", ServeDir::new(&config.frontend_dir))
        .layer(TraceLayer::new_for_http())
        .layer(RequestBodyLimitLayer::new(config.max_upload_size))
        .layer(cors)
        .with_state(state);

    // Start server
    let addr = config.server_addr();
    tracing::info!("Server listening on http://{}", addr);
    tracing::info!("Frontend served from: {}", config.frontend_dir);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

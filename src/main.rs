mod db;
mod errors;
mod handlers;
mod models;
mod parser;

use crate::db::Database;
use crate::handlers::{
    delete_track_point, get_devices, get_raw_packet, get_track_point, get_track_points,
    health_check, not_found, upload_location_data,
};
use axum::{
    http::Method,
    routing::{delete, get, post},
    Router,
};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;
use utoipa::{OpenApi, Modify};
use utoipa_swagger_ui::SwaggerUi;

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.get_or_insert_with(Default::default);
        components.add_security_scheme(
            "api_key",
            utoipa::openapi::security::SecurityScheme::ApiKey(
                utoipa::openapi::security::ApiKey::Header(utoipa::openapi::security::ApiKeyHeader::new(
                    "X-API-Key",
                )),
            ),
        );
    }
}

#[derive(OpenApi)]
#[openapi(
    paths(
        health_check,
        upload_location_data,
        get_track_points,
        get_track_point,
        delete_track_point,
        get_devices,
        get_raw_packet,
    ),
    components(
        schemas(
            models::LocationDataPacket,
            models::DataFormat,
            models::TrackPoint,
            models::TrackQuery,
            models::SortOrder,
            models::TrackListResponse,
            models::UploadResponse,
            models::ApiResponse<serde_json::Value>,
            models::ParsedPacket,
            models::BinaryLocationPoint,
            handlers::HealthResponse,
        )
    ),
    tags(
        (name = "System", description = "System management endpoints"),
        (name = "Location", description = "Location data upload endpoints"),
        (name = "Track", description = "Track point query endpoints"),
        (name = "Device", description = "Device management endpoints"),
    ),
    info(
        title = "Location Tracker API",
        version = "0.1.0",
        description = "Rust + Axum service for receiving and storing offline location tracking data from IoT devices",
    ),
    modifiers(&SecurityAddon)
)]
struct ApiDoc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::DEBUG)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:location.db".to_string());
    let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());

    let db = Database::new(&database_url).await?;

    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE, Method::OPTIONS])
        .allow_origin(Any)
        .allow_headers(Any);

    let api_routes = Router::new()
        .route("/location/upload", post(upload_location_data))
        .route("/track/points", get(get_track_points))
        .route("/track/points/:id", get(get_track_point))
        .route("/track/points/:id", delete(delete_track_point))
        .route("/devices", get(get_devices))
        .route("/packets/:id", get(get_raw_packet));

    let app = Router::new()
        .route("/health", get(health_check))
        .nest("/api/v1", api_routes)
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .fallback(not_found)
        .layer(cors)
        .with_state(db);

    let addr: SocketAddr = format!("{}:{}", host, port).parse()?;
    info!("Starting server on {}", addr);

    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

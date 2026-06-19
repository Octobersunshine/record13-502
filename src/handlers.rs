use crate::db::Database;
use crate::errors::{AppError, AppResult};
use crate::models::{
    ApiResponse, LocationDataPacket, TrackListResponse, TrackPoint, TrackQuery, UploadResponse,
};
use crate::parser::PacketParser;
use axum::{
    extract::{Path, Query, State},
    Json,
    http::StatusCode,
};
use serde_json::json;
use tracing::{debug, info, warn};
use uuid::Uuid;
use utoipa::ToSchema;

#[derive(Debug, serde::Deserialize, ToSchema)]
pub struct HealthResponse {
    #[schema(example = "ok")]
    pub status: String,
    #[schema(example = "0.1.0")]
    pub version: String,
}

#[utoipa::path(
    get,
    path = "/health",
    responses(
        (status = 200, description = "Service health check", body = HealthResponse)
    ),
    tag = "System"
)]
pub async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

#[utoipa::path(
    post,
    path = "/api/v1/location/upload",
    request_body = LocationDataPacket,
    responses(
        (status = 200, description = "Location data uploaded successfully", body = UploadResponse),
        (status = 400, description = "Invalid request data"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Location"
)]
pub async fn upload_location_data(
    State(db): State<Database>,
    Json(packet): Json<LocationDataPacket>,
) -> AppResult<(StatusCode, Json<UploadResponse>)> {
    info!(
        "Received location data from device: {}, format: {:?}",
        packet.device_id, packet.data_format
    );

    if packet.device_id.is_empty() {
        return Err(AppError::ValidationError("device_id cannot be empty".to_string()));
    }

    if packet.payload.is_empty() {
        return Err(AppError::ValidationError("payload cannot be empty".to_string()));
    }

    let raw_packet_id = db.insert_raw_packet(&packet).await?;

    let points = match PacketParser::parse_packet(&packet, raw_packet_id).await;

    let (points_stored, status, message) = match points {
        Ok(parsed_points) => {
            let points_received = parsed_points.len();
            debug!("Successfully parsed {} points", points_received);

            let stored = db
                .insert_track_points(&parsed_points, raw_packet_id)
                .await?;

            db.mark_packet_parsed(raw_packet_id, true, None).await?;

            (
                stored,
                StatusCode::OK,
                format!("Data received and stored successfully"),
            )
        }
        Err(e) => {
            warn!("Parse error for packet {}: {:?}", raw_packet_id, e);
            db.mark_packet_parsed(raw_packet_id, false, Some(&e.to_string()))
                .await?;
            return Err(e);
        }
    };

    let response = UploadResponse {
        packet_id: raw_packet_id,
        points_received: points_stored,
        points_stored,
        status: "success".to_string(),
        message,
    };

    Ok((status, Json(response)))
}

#[utoipa::path(
    get,
    path = "/api/v1/track/points",
    params(TrackQuery),
    responses(
        (status = 200, description = "List of track points", body = TrackListResponse),
        (status = 500, description = "Internal server error")
    ),
    tag = "Track"
)]
pub async fn get_track_points(
    State(db): State<Database>,
    Query(query): Query<TrackQuery>,
) -> AppResult<Json<TrackListResponse>> {
    debug!("Querying track points: {:?}", query);
    let result = db.get_track_points(query).await?;
    Ok(Json(result))
}

#[utoipa::path(
    get,
    path = "/api/v1/track/points/{id}",
    responses(
        (status = 200, description = "Track point found", body = TrackPoint),
        (status = 404, description = "Track point not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Track"
)]
pub async fn get_track_point(
    State(db): State<Database>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<TrackPoint>> {
    debug!("Getting track point: {}", id);

    match db.get_track_point_by_id(id).await? {
        Some(point) => Ok(Json(point)),
        None => Err(AppError::NotFound(format!(
            "Track point not found"
        ))),
    }
}

#[utoipa::path(
    delete,
    path = "/api/v1/track/points/{id}",
    responses(
        (status = 200, description = "Track point deleted"),
        (status = 404, description = "Track point not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Track"
)]
pub async fn delete_track_point(
    State(db): State<Database>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<ApiResponse<()>>> {
    debug!("Deleting track point: {}", id);

    match db.delete_track_point(id).await? {
        true => Ok(Json(ApiResponse {
            success: true,
            message: "Track point deleted successfully".to_string(),
            data: None,
        })),
        false => Err(AppError::NotFound(format!(
            "Track point not found"
        ))),
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/devices",
    responses(
        (status = 200, description = "List of device IDs"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Device"
)]
pub async fn get_devices(
    State(db): State<Database>,
) -> AppResult<Json<ApiResponse<Vec<String>>>> {
    let devices = db.get_device_list().await?;
    Ok(Json(ApiResponse {
        success: true,
        message: format!("Found {} devices", devices.len()),
        data: Some(devices),
    })
}

#[utoipa::path(
    get,
    path = "/api/v1/packets/{id}",
    responses(
        (status = 200, description = "Raw packet found"),
        (status = 404, description = "Raw packet not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Location"
)]
pub async fn get_raw_packet(
    State(db): State<Database>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<crate::models::RawPacketRecord>> {
    debug!("Getting raw packet: {}", id);

    match db.get_raw_packet(id).await? {
        Some(packet) => Ok(Json(packet)),
        None => Err(AppError::NotFound(format!(
            "Raw packet not found"
        ))),
    }
}

pub async fn not_found() -> impl axum::response::IntoResponse {
    (
        StatusCode::NOT_FOUND,
        Json(json!({
            "success": false,
            "error": "Not Found",
            "message": "The requested resource was not found",
            "status_code": 404,
        })),
    )
}

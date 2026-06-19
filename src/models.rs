use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct LocationDataPacket {
    #[schema(example = "dev_001")]
    pub device_id: String,

    #[schema(example = "GPS_TRACKER_V2")]
    pub device_type: Option<String>,

    #[schema(example = "1.0.0")]
    pub firmware_version: Option<String>,

    #[schema(example = "2024-01-15T10:30:00Z")]
    pub upload_time: DateTime<Utc>,

    #[schema(example = "json")]
    pub data_format: DataFormat,

    #[schema(example = "base64_encoded_data...")]
    pub payload: String,

    pub checksum: Option<String>,

    pub signature: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DataFormat {
    Json,
    Binary,
    Hex,
    Csv,
    Protobuf,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ParsedPacket {
    pub device_id: String,
    pub upload_time: DateTime<Utc>,
    pub track_points: Vec<TrackPoint>,
    pub raw_packet_id: Uuid,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, FromRow, Clone)]
pub struct TrackPoint {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Uuid>,

    #[schema(example = "dev_001")]
    pub device_id: String,

    #[schema(example = 39.9042)]
    pub latitude: f64,

    #[schema(example = 116.4074)]
    pub longitude: f64,

    #[schema(example = 45.5)]
    pub altitude: Option<f64>,

    #[schema(example = 60.0)]
    pub speed: Option<f64>,

    #[schema(example = 180.0)]
    pub heading: Option<f64>,

    #[schema(example = 8)]
    pub satellites: Option<i32>,

    #[schema(example = 10.0)]
    pub hdop: Option<f64>,

    #[schema(example = "2024-01-15T10:25:30Z")]
    pub timestamp: DateTime<Utc>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "GPS")]
    pub location_source: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub accuracy: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub battery_level: Option<f32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_data: Option<serde_json::Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct RawPacketRecord {
    pub id: Uuid,
    pub device_id: String,
    pub device_type: Option<String>,
    pub firmware_version: Option<String>,
    pub upload_time: DateTime<Utc>,
    pub data_format: String,
    pub payload: String,
    pub checksum: Option<String>,
    pub signature: Option<String>,
    pub parsed: bool,
    pub parse_error: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UploadResponse {
    #[schema(example = "550e8400-e29b-41d4-a716-446655440000")]
    pub packet_id: Uuid,

    #[schema(example = 10)]
    pub points_received: usize,

    #[schema(example = 10)]
    pub points_stored: usize,

    #[schema(example = "success")]
    pub status: String,

    #[schema(example = "Data received and stored successfully")]
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub message: String,
    pub data: Option<T>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SortOrder {
    #[schema(example = "asc")]
    Asc,
    #[schema(example = "desc")]
    Desc,
}

impl Default for SortOrder {
    fn default() -> Self {
        SortOrder::Asc
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TrackQuery {
    pub device_id: Option<String>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,

    #[schema(example = "asc", default = "asc")]
    #[serde(default)]
    pub order: SortOrder,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TrackListResponse {
    pub total: i64,
    pub points: Vec<TrackPoint>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BinaryLocationPoint {
    pub timestamp: u32,
    pub latitude: i32,
    pub longitude: i32,
    pub altitude: i16,
    pub speed: u8,
    pub heading: u8,
    pub satellites: u8,
    pub flags: u8,
}

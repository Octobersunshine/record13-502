use chrono::{TimeZone, Utc};
use location_tracker::models::{DataFormat, LocationDataPacket};
use serde_json::json;

fn create_test_json_payload() -> String {
    let points = vec![
        json!({
            "latitude": 39.9042,
            "longitude": 116.4074,
            "timestamp": "2024-01-15T10:25:30Z",
            "altitude": 45.5,
            "speed": 60.0,
            "heading": 180.0,
            "satellites": 8,
            "source": "GPS"
        }),
        json!({
            "latitude": 39.9050,
            "longitude": 116.4080,
            "timestamp": "2024-01-15T10:25:35Z",
            "altitude": 46.0,
            "speed": 62.0,
            "heading": 185.0,
            "satellites": 9,
            "source": "GPS"
        }),
    ];
    serde_json::to_string(&points).unwrap()
}

fn create_test_binary_payload() -> Vec<u8> {
    let mut data = Vec::new();

    let timestamp = 1705313130u32;
    let lat = (39.9042 * 1_000_000.0) as i32;
    let lng = (116.4074 * 1_000_000.0) as i32;
    let alt = (45.5 * 10.0) as i16;
    let speed = (60.0 / 0.5) as u8;
    let heading = (180.0 / 1.4117647) as u8;
    let satellites = 8u8;
    let flags = 0x01u8;

    data.extend_from_slice(&timestamp.to_le_bytes());
    data.extend_from_slice(&lat.to_le_bytes());
    data.extend_from_slice(&lng.to_le_bytes());
    data.extend_from_slice(&alt.to_le_bytes());
    data.push(speed);
    data.push(heading);
    data.push(satellites);
    data.push(flags);

    let timestamp2 = 1705313135u32;
    let lat2 = (39.9050 * 1_000_000.0) as i32;
    let lng2 = (116.4080 * 1_000_000.0) as i32;
    data.extend_from_slice(&timestamp2.to_le_bytes());
    data.extend_from_slice(&lat2.to_le_bytes());
    data.extend_from_slice(&lng2.to_le_bytes());
    data.extend_from_slice(&alt.to_le_bytes());
    data.push(speed);
    data.push(heading);
    data.push(satellites);
    data.push(flags);

    data
}

fn create_test_csv_payload() -> String {
    "latitude,longitude,timestamp,altitude,speed,satellites\n\
     39.9042,116.4074,2024-01-15T10:25:30Z,45.5,60.0,8\n\
     39.9050,116.4080,2024-01-15T10:25:35Z,46.0,62.0,9"
        .to_string()
}

#[tokio::test]
async fn test_parse_json_packet() {
    use location_tracker::parser::PacketParser;
    use uuid::Uuid;

    let payload = create_test_json_payload();

    let packet = LocationDataPacket {
        device_id: "test_device_001".to_string(),
        device_type: Some("GPS_TRACKER".to_string()),
        firmware_version: Some("1.0.0".to_string()),
        upload_time: Utc::now(),
        data_format: DataFormat::Json,
        payload,
        checksum: None,
        signature: None,
    };

    let raw_packet_id = Uuid::new_v4();
    let result = PacketParser::parse_packet(&packet, raw_packet_id).await;

    assert!(result.is_ok());
    let points = result.unwrap();
    assert_eq!(points.len(), 2);
    assert_eq!(points[0].device_id, "test_device_001");
    assert!((points[0].latitude - 39.9042).abs() < 0.0001);
    assert!((points[0].longitude - 116.4074).abs() < 0.0001);
    assert_eq!(points[0].altitude, Some(45.5));
    assert_eq!(points[0].speed, Some(60.0));
    assert_eq!(points[0].satellites, Some(8));
    assert_eq!(points[0].location_source.as_deref(), Some("GPS"));
}

#[tokio::test]
async fn test_parse_binary_packet() {
    use location_tracker::parser::PacketParser;
    use uuid::Uuid;

    let binary_data = create_test_binary_payload();
    let payload = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &binary_data);

    let packet = LocationDataPacket {
        device_id: "test_device_002".to_string(),
        device_type: Some("GPS_TRACKER".to_string()),
        firmware_version: Some("1.0.0".to_string()),
        upload_time: Utc::now(),
        data_format: DataFormat::Binary,
        payload,
        checksum: None,
        signature: None,
    };

    let raw_packet_id = Uuid::new_v4();
    let result = PacketParser::parse_packet(&packet, raw_packet_id).await;

    assert!(result.is_ok());
    let points = result.unwrap();
    assert_eq!(points.len(), 2);
    assert_eq!(points[0].device_id, "test_device_002");
    assert!((points[0].latitude - 39.9042).abs() < 0.0001);
    assert!((points[0].longitude - 116.4074).abs() < 0.0001);
    assert_eq!(points[0].location_source.as_deref(), Some("GPS"));

    let expected_ts = Utc.timestamp_opt(1705313130, 0).single().unwrap();
    assert_eq!(points[0].timestamp, expected_ts);
}

#[tokio::test]
async fn test_parse_hex_packet() {
    use location_tracker::parser::PacketParser;
    use uuid::Uuid;

    let binary_data = create_test_binary_payload();
    let payload = hex::encode(&binary_data);

    let packet = LocationDataPacket {
        device_id: "test_device_003".to_string(),
        device_type: Some("GPS_TRACKER".to_string()),
        firmware_version: Some("1.0.0".to_string()),
        upload_time: Utc::now(),
        data_format: DataFormat::Hex,
        payload,
        checksum: None,
        signature: None,
    };

    let raw_packet_id = Uuid::new_v4();
    let result = PacketParser::parse_packet(&packet, raw_packet_id).await;

    assert!(result.is_ok());
    let points = result.unwrap();
    assert_eq!(points.len(), 2);
}

#[tokio::test]
async fn test_parse_csv_packet() {
    use location_tracker::parser::PacketParser;
    use uuid::Uuid;

    let payload = create_test_csv_payload();

    let packet = LocationDataPacket {
        device_id: "test_device_004".to_string(),
        device_type: Some("GPS_TRACKER".to_string()),
        firmware_version: Some("1.0.0".to_string()),
        upload_time: Utc::now(),
        data_format: DataFormat::Csv,
        payload,
        checksum: None,
        signature: None,
    };

    let raw_packet_id = Uuid::new_v4();
    let result = PacketParser::parse_packet(&packet, raw_packet_id).await;

    assert!(result.is_ok());
    let points = result.unwrap();
    assert_eq!(points.len(), 2);
    assert_eq!(points[0].device_id, "test_device_004");
    assert!((points[0].latitude - 39.9042).abs() < 0.0001);
    assert!((points[0].longitude - 116.4074).abs() < 0.0001);
    assert_eq!(points[0].altitude, Some(45.5));
    assert_eq!(points[0].speed, Some(60.0));
    assert_eq!(points[0].satellites, Some(8));
}

#[tokio::test]
async fn test_database_operations() {
    use location_tracker::db::Database;
    use location_tracker::models::{TrackPoint, TrackQuery};
    use chrono::Duration;
    use uuid::Uuid;

    let db = Database::new("sqlite::memory:").await.unwrap();

    let ts = Utc::now();
    let point = TrackPoint {
        id: None,
        device_id: "db_test_001".to_string(),
        latitude: 39.9042,
        longitude: 116.4074,
        altitude: Some(45.5),
        speed: Some(60.0),
        heading: Some(180.0),
        satellites: Some(8),
        hdop: Some(2.5),
        timestamp: ts,
        location_source: Some("GPS".to_string()),
        accuracy: Some(5.0),
        battery_level: Some(85.0),
        extra_data: Some(json!({"test": "value"})),
        created_at: None,
    };

    let packet_id = Uuid::new_v4();
    let inserted = db.insert_track_points(&[point.clone()], packet_id).await.unwrap();
    assert_eq!(inserted, 1);

    let query = TrackQuery {
        device_id: Some("db_test_001".to_string()),
        start_time: None,
        end_time: None,
        limit: Some(10),
        offset: None,
    };

    let result = db.get_track_points(query).await.unwrap();
    assert_eq!(result.total, 1);
    assert_eq!(result.points.len(), 1);
    assert_eq!(result.points[0].device_id, "db_test_001");
    assert!((result.points[0].latitude - 39.9042).abs() < 0.0001);

    let devices = db.get_device_list().await.unwrap();
    assert!(devices.contains(&"db_test_001".to_string()));

    let point_id = result.points[0].id.unwrap();
    let single = db.get_track_point_by_id(point_id).await.unwrap();
    assert!(single.is_some());
    assert_eq!(single.unwrap().device_id, "db_test_001");

    let deleted = db.delete_track_point(point_id).await.unwrap();
    assert!(deleted);

    let single = db.get_track_point_by_id(point_id).await.unwrap();
    assert!(single.is_none());
}

#[test]
fn test_validation_errors() {
    use location_tracker::errors::AppError;

    let invalid_json = "not a json";
    let result: Result<serde_json::Value, _> = serde_json::from_str(invalid_json);
    assert!(result.is_err());

    let err = AppError::ParseError("Invalid JSON".to_string());
    assert_eq!(err.to_string(), "Parse error: Invalid JSON");

    let err = AppError::ValidationError("device_id cannot be empty".to_string());
    assert_eq!(err.to_string(), "Validation error: device_id cannot be empty");
}

#[tokio::test]
async fn test_raw_packet_storage() {
    use location_tracker::db::Database;
    use location_tracker::models::LocationDataPacket;

    let db = Database::new("sqlite::memory:").await.unwrap();

    let packet = LocationDataPacket {
        device_id: "raw_test_001".to_string(),
        device_type: Some("TEST_DEVICE".to_string()),
        firmware_version: Some("2.0.0".to_string()),
        upload_time: Utc::now(),
        data_format: DataFormat::Json,
        payload: create_test_json_payload(),
        checksum: None,
        signature: None,
    };

    let packet_id = db.insert_raw_packet(&packet).await.unwrap();

    db.mark_packet_parsed(packet_id, true, None).await.unwrap();

    let stored = db.get_raw_packet(packet_id).await.unwrap();
    assert!(stored.is_some());
    let stored = stored.unwrap();
    assert_eq!(stored.device_id, "raw_test_001");
    assert_eq!(stored.data_format, "json");
    assert_eq!(stored.parsed, true);
    assert!(stored.parse_error.is_none());
}

#[tokio::test]
async fn test_time_range_query() {
    use location_tracker::db::Database;
    use location_tracker::models::{TrackPoint, TrackQuery};
    use chrono::Duration;

    let db = Database::new("sqlite::memory:").await.unwrap();

    let now = Utc::now();
    let packet_id = Uuid::new_v4();

    let points = vec![
        TrackPoint {
            id: None,
            device_id: "time_test_001".to_string(),
            latitude: 39.9042,
            longitude: 116.4074,
            altitude: None,
            speed: None,
            heading: None,
            satellites: None,
            hdop: None,
            timestamp: now - Duration::hours(2),
            location_source: None,
            accuracy: None,
            battery_level: None,
            extra_data: None,
            created_at: None,
        },
        TrackPoint {
            id: None,
            device_id: "time_test_001".to_string(),
            latitude: 39.9050,
            longitude: 116.4080,
            altitude: None,
            speed: None,
            heading: None,
            satellites: None,
            hdop: None,
            timestamp: now - Duration::hours(1),
            location_source: None,
            accuracy: None,
            battery_level: None,
            extra_data: None,
            created_at: None,
        },
        TrackPoint {
            id: None,
            device_id: "time_test_001".to_string(),
            latitude: 39.9060,
            longitude: 116.4090,
            altitude: None,
            speed: None,
            heading: None,
            satellites: None,
            hdop: None,
            timestamp: now,
            location_source: None,
            accuracy: None,
            battery_level: None,
            extra_data: None,
            created_at: None,
        },
    ];

    db.insert_track_points(&points, packet_id).await.unwrap();

    let query = TrackQuery {
        device_id: Some("time_test_001".to_string()),
        start_time: Some(now - Duration::hours(90)),
        end_time: Some(now - Duration::minutes(30)),
        limit: None,
        offset: None,
    };

    let result = db.get_track_points(query).await.unwrap();
    assert_eq!(result.total, 2);

    let query_all = TrackQuery {
        device_id: Some("time_test_001".to_string()),
        start_time: None,
        end_time: None,
        limit: None,
        offset: None,
    };

    let result_all = db.get_track_points(query_all).await.unwrap();
    assert_eq!(result_all.total, 3);
}

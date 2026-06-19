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

#[tokio::test]
async fn test_parser_sorts_unordered_timestamps() {
    use location_tracker::parser::PacketParser;
    use location_tracker::models::{DataFormat, LocationDataPacket};
    use chrono::{Duration, Utc};
    use uuid::Uuid;
    use serde_json::json;

    let now = Utc::now();

    let unordered_points = json!([
        {
            "latitude": 39.9060,
            "longitude": 116.4090,
            "timestamp": (now + Duration::seconds(20)).to_rfc3339(),
        },
        {
            "latitude": 39.9042,
            "longitude": 116.4074,
            "timestamp": now.to_rfc3339(),
        },
        {
            "latitude": 39.9050,
            "longitude": 116.4080,
            "timestamp": (now + Duration::seconds(10)).to_rfc3339(),
        },
        {
            "latitude": 39.9070,
            "longitude": 116.4100,
            "timestamp": (now + Duration::seconds(5)).to_rfc3339(),
        },
    ]);

    let packet = LocationDataPacket {
        device_id: "sort_test_001".to_string(),
        device_type: Some("TEST_DEVICE".to_string()),
        firmware_version: Some("1.0.0".to_string()),
        upload_time: Utc::now(),
        data_format: DataFormat::Json,
        payload: serde_json::to_string(&unordered_points).unwrap(),
        checksum: None,
        signature: None,
    };

    let raw_packet_id = Uuid::new_v4();
    let result = PacketParser::parse_packet(&packet, raw_packet_id).await;

    assert!(result.is_ok());
    let points = result.unwrap();
    assert_eq!(points.len(), 4);

    assert!(points[0].timestamp <= points[1].timestamp);
    assert!(points[1].timestamp <= points[2].timestamp);
    assert!(points[2].timestamp <= points[3].timestamp);

    assert_eq!(points[0].latitude, 39.9042);
    assert_eq!(points[1].latitude, 39.9070);
    assert_eq!(points[2].latitude, 39.9050);
    assert_eq!(points[3].latitude, 39.9060);
}

#[tokio::test]
async fn test_database_query_sort_order() {
    use location_tracker::db::Database;
    use location_tracker::models::{SortOrder, TrackPoint, TrackQuery};
    use chrono::{Duration, Utc};
    use uuid::Uuid;

    let db = Database::new("sqlite::memory:").await.unwrap();

    let now = Utc::now();
    let packet_id = Uuid::new_v4();

    let points = vec![
        TrackPoint {
            id: None,
            device_id: "sort_test_002".to_string(),
            latitude: 1.0,
            longitude: 1.0,
            altitude: None,
            speed: None,
            heading: None,
            satellites: None,
            hdop: None,
            timestamp: now + Duration::hours(3),
            location_source: None,
            accuracy: None,
            battery_level: None,
            extra_data: None,
            created_at: None,
        },
        TrackPoint {
            id: None,
            device_id: "sort_test_002".to_string(),
            latitude: 2.0,
            longitude: 2.0,
            altitude: None,
            speed: None,
            heading: None,
            satellites: None,
            hdop: None,
            timestamp: now + Duration::hours(1),
            location_source: None,
            accuracy: None,
            battery_level: None,
            extra_data: None,
            created_at: None,
        },
        TrackPoint {
            id: None,
            device_id: "sort_test_002".to_string(),
            latitude: 3.0,
            longitude: 3.0,
            altitude: None,
            speed: None,
            heading: None,
            satellites: None,
            hdop: None,
            timestamp: now + Duration::hours(2),
            location_source: None,
            accuracy: None,
            battery_level: None,
            extra_data: None,
            created_at: None,
        },
    ];

    db.insert_track_points(&points, packet_id).await.unwrap();

    let query_asc = TrackQuery {
        device_id: Some("sort_test_002".to_string()),
        start_time: None,
        end_time: None,
        limit: None,
        offset: None,
        order: SortOrder::Asc,
    };

    let result_asc = db.get_track_points(query_asc).await.unwrap();
    assert_eq!(result_asc.total, 3);
    assert_eq!(result_asc.points[0].latitude, 2.0);
    assert_eq!(result_asc.points[1].latitude, 3.0);
    assert_eq!(result_asc.points[2].latitude, 1.0);
    assert!(result_asc.points[0].timestamp < result_asc.points[1].timestamp);
    assert!(result_asc.points[1].timestamp < result_asc.points[2].timestamp);

    let query_desc = TrackQuery {
        device_id: Some("sort_test_002".to_string()),
        start_time: None,
        end_time: None,
        limit: None,
        offset: None,
        order: SortOrder::Desc,
    };

    let result_desc = db.get_track_points(query_desc).await.unwrap();
    assert_eq!(result_desc.total, 3);
    assert_eq!(result_desc.points[0].latitude, 1.0);
    assert_eq!(result_desc.points[1].latitude, 3.0);
    assert_eq!(result_desc.points[2].latitude, 2.0);
    assert!(result_desc.points[0].timestamp > result_desc.points[1].timestamp);
    assert!(result_desc.points[1].timestamp > result_desc.points[2].timestamp);

    let query_default = TrackQuery {
        device_id: Some("sort_test_002".to_string()),
        start_time: None,
        end_time: None,
        limit: None,
        offset: None,
        order: Default::default(),
    };

    let result_default = db.get_track_points(query_default).await.unwrap();
    assert_eq!(result_default.points[0].latitude, 2.0);
    assert_eq!(result_default.points[1].latitude, 3.0);
    assert_eq!(result_default.points[2].latitude, 1.0);
}

#[tokio::test]
async fn test_track_playback_correct_order() {
    use location_tracker::db::Database;
    use location_tracker::models::{TrackPoint, TrackQuery, SortOrder};
    use chrono::{Duration, Utc};
    use uuid::Uuid;

    let db = Database::new("sqlite::memory:").await.unwrap();

    let now = Utc::now();
    let packet_id = Uuid::new_v4();

    let route_points = vec![
        ("起点", 39.9000, 116.4000, Duration::seconds(0)),
        ("点A", 39.9010, 116.4010, Duration::seconds(10)),
        ("点B", 39.9020, 116.4020, Duration::seconds(20)),
        ("点C", 39.9030, 116.4030, Duration::seconds(30)),
        ("点D", 39.9040, 116.4040, Duration::seconds(40)),
        ("终点", 39.9050, 116.4050, Duration::seconds(50)),
    ];

    let mut points: Vec<TrackPoint> = route_points
        .iter()
        .map(|(_, lat, lng, offset)| TrackPoint {
            id: None,
            device_id: "playback_test_001".to_string(),
            latitude: *lat,
            longitude: *lng,
            altitude: None,
            speed: Some(10.0),
            heading: Some(45.0),
            satellites: Some(8),
            hdop: None,
            timestamp: now + *offset,
            location_source: Some("GPS".to_string()),
            accuracy: None,
            battery_level: None,
            extra_data: None,
            created_at: None,
        })
        .collect();

    points.reverse();

    db.insert_track_points(&points, packet_id).await.unwrap();

    let query = TrackQuery {
        device_id: Some("playback_test_001".to_string()),
        start_time: None,
        end_time: None,
        limit: None,
        offset: None,
        order: SortOrder::Asc,
    };

    let result = db.get_track_points(query).await.unwrap();
    assert_eq!(result.total, 6);

    assert!((result.points[0].latitude - 39.9000).abs() < 0.0001);
    assert!((result.points[1].latitude - 39.9010).abs() < 0.0001);
    assert!((result.points[2].latitude - 39.9020).abs() < 0.0001);
    assert!((result.points[3].latitude - 39.9030).abs() < 0.0001);
    assert!((result.points[4].latitude - 39.9040).abs() < 0.0001);
    assert!((result.points[5].latitude - 39.9050).abs() < 0.0001);

    for i in 0..result.points.len() - 1 {
        assert!(
            result.points[i].timestamp < result.points[i + 1].timestamp,
            "Point {} should have earlier timestamp than point {}",
            i,
            i + 1
        );
    }

    let distances: Vec<f64> = result.points.windows(2)
        .map(|pair| {
            let dlat = pair[1].latitude - pair[0].latitude;
            let dlng = pair[1].longitude - pair[0].longitude;
            (dlat * dlat + dlng * dlng).sqrt()
        })
        .collect();

    for d in distances {
        assert!((d - 0.0014142).abs() < 0.0001, "Distance should be consistent for route playback");
    }
}

#[test]
fn test_sort_order_default_is_asc() {
    use location_tracker::models::SortOrder;

    let default_order: SortOrder = Default::default();
    assert_eq!(default_order, SortOrder::Asc);
}

#[test]
fn test_sort_order_serde() {
    use location_tracker::models::SortOrder;
    use serde_json;

    let asc_json = serde_json::to_string(&SortOrder::Asc).unwrap();
    assert_eq!(asc_json, "\"asc\"");

    let desc_json = serde_json::to_string(&SortOrder::Desc).unwrap();
    assert_eq!(desc_json, "\"desc\"");

    let asc: SortOrder = serde_json::from_str("\"asc\"").unwrap();
    assert_eq!(asc, SortOrder::Asc);

    let desc: SortOrder = serde_json::from_str("\"desc\"").unwrap();
    assert_eq!(desc, SortOrder::Desc);
}

#[tokio::test]
async fn test_track_points_with_distance_insertion() {
    use location_tracker::db::Database;
    use location_tracker::models::{SortOrder, TrackPoint, TrackQuery};
    use chrono::{Duration, Utc};
    use uuid::Uuid;

    let db = Database::new("sqlite::memory:").await.unwrap();

    let now = Utc::now();
    let packet_id = Uuid::new_v4();

    let base_lat = 39.9000;
    let base_lng = 116.4000;

    let points: Vec<TrackPoint> = (0..5)
        .map(|i| TrackPoint {
            id: None,
            device_id: "dist_test_001".to_string(),
            latitude: base_lat + (i as f64) * 0.001,
            longitude: base_lng + (i as f64) * 0.001,
            altitude: None,
            speed: Some(30.0 + i as f64),
            heading: Some(45.0),
            satellites: Some(8),
            hdop: None,
            timestamp: now + Duration::seconds(i * 10),
            location_source: Some("GPS".to_string()),
            accuracy: Some(5.0),
            battery_level: Some(85.0),
            extra_data: None,
            distance: None,
            cumulative_distance: None,
            created_at: None,
        })
        .collect();

    let inserted = db.insert_track_points(&points, packet_id).await.unwrap();
    assert_eq!(inserted, 5);

    let query = TrackQuery {
        device_id: Some("dist_test_001".to_string()),
        start_time: None,
        end_time: None,
        limit: None,
        offset: None,
        order: SortOrder::Asc,
    };

    let result = db.get_track_points(query).await.unwrap();
    assert_eq!(result.total, 5);

    assert!(result.points[0].distance.unwrap_or(0.0) < 0.01);

    for i in 1..result.points.len() {
        let dist = result.points[i].distance.unwrap();
        assert!(dist > 100.0 && dist < 200.0, "Distance at {} should be ~141m, got {}", i, dist);
    }

    for i in 1..result.points.len() {
        let cum = result.points[i].cumulative_distance.unwrap();
        let prev_cum = result.points[i - 1].cumulative_distance.unwrap();
        assert!(cum > prev_cum, "Cumulative distance should increase at index {}", i);
    }

    assert!(result.total_distance_meters > 400.0 && result.total_distance_meters < 800.0);
    assert!(result.total_distance_formatted.contains("km") || result.total_distance_formatted.contains("m"));
}

#[tokio::test]
async fn test_mileage_calculation_single_device() {
    use location_tracker::db::Database;
    use location_tracker::models::{MileageQuery, TrackPoint};
    use chrono::{Duration, Utc};
    use uuid::Uuid;

    let db = Database::new("sqlite::memory:").await.unwrap();

    let now = Utc::now();
    let packet_id = Uuid::new_v4();

    let points: Vec<TrackPoint> = (0..10)
        .map(|i| TrackPoint {
            id: None,
            device_id: "mileage_001".to_string(),
            latitude: 39.9000 + (i as f64) * 0.001,
            longitude: 116.4000 + (i as f64) * 0.001,
            altitude: None,
            speed: Some(50.0),
            heading: Some(45.0),
            satellites: Some(8),
            hdop: None,
            timestamp: now + Duration::seconds(i * 10),
            location_source: Some("GPS".to_string()),
            accuracy: Some(5.0),
            battery_level: Some(85.0),
            extra_data: None,
            distance: None,
            cumulative_distance: None,
            created_at: None,
        })
        .collect();

    db.insert_track_points(&points, packet_id).await.unwrap();

    let query = MileageQuery {
        device_id: Some("mileage_001".to_string()),
        start_time: None,
        end_time: None,
        max_speed_kmh: None,
        min_accuracy: None,
        stop_timeout_seconds: None,
    };

    let stats = db.calculate_mileage(query).await.unwrap();
    assert_eq!(stats.len(), 1);
    assert_eq!(stats[0].device_id, "mileage_001");
    assert!(stats[0].point_count >= 9);
    assert!(stats[0].total_distance_meters > 1000.0);
    assert!(stats[0].start_time.is_some());
    assert!(stats[0].end_time.is_some());
    assert!(stats[0].avg_speed_kmh.is_some());
    assert!(stats[0].max_speed_kmh.is_some());
    assert!(stats[0].moving_duration_minutes.is_some());
}

#[tokio::test]
async fn test_mileage_calculation_multiple_devices() {
    use location_tracker::db::Database;
    use location_tracker::models::{MileageQuery, TrackPoint};
    use chrono::{Duration, Utc};
    use uuid::Uuid;

    let db = Database::new("sqlite::memory:").await.unwrap();

    let now = Utc::now();
    let packet_id1 = Uuid::new_v4();
    let packet_id2 = Uuid::new_v4();

    let points1: Vec<TrackPoint> = (0..5)
        .map(|i| TrackPoint {
            id: None,
            device_id: "mileage_multi_001".to_string(),
            latitude: 39.9000 + (i as f64) * 0.001,
            longitude: 116.4000 + (i as f64) * 0.001,
            altitude: None,
            speed: Some(40.0),
            heading: Some(45.0),
            satellites: Some(8),
            hdop: None,
            timestamp: now + Duration::seconds(i * 10),
            location_source: Some("GPS".to_string()),
            accuracy: Some(5.0),
            battery_level: Some(85.0),
            extra_data: None,
            distance: None,
            cumulative_distance: None,
            created_at: None,
        })
        .collect();

    let points2: Vec<TrackPoint> = (0..5)
        .map(|i| TrackPoint {
            id: None,
            device_id: "mileage_multi_002".to_string(),
            latitude: 31.2300 + (i as f64) * 0.001,
            longitude: 121.4700 + (i as f64) * 0.001,
            altitude: None,
            speed: Some(60.0),
            heading: Some(90.0),
            satellites: Some(10),
            hdop: None,
            timestamp: now + Duration::seconds(i * 10),
            location_source: Some("GPS".to_string()),
            accuracy: Some(3.0),
            battery_level: Some(90.0),
            extra_data: None,
            distance: None,
            cumulative_distance: None,
            created_at: None,
        })
        .collect();

    db.insert_track_points(&points1, packet_id1).await.unwrap();
    db.insert_track_points(&points2, packet_id2).await.unwrap();

    let query = MileageQuery {
        device_id: None,
        start_time: None,
        end_time: None,
        max_speed_kmh: None,
        min_accuracy: None,
        stop_timeout_seconds: None,
    };

    let stats = db.calculate_mileage(query).await.unwrap();
    assert_eq!(stats.len(), 2);

    let device_ids: Vec<&String> = stats.iter().map(|s| &s.device_id).collect();
    assert!(device_ids.contains(&&"mileage_multi_001".to_string()));
    assert!(device_ids.contains(&&"mileage_multi_002".to_string()));
}

#[tokio::test]
async fn test_mileage_with_speed_filter() {
    use location_tracker::db::Database;
    use location_tracker::models::{MileageQuery, TrackPoint};
    use chrono::{Duration, Utc};
    use uuid::Uuid;

    let db = Database::new("sqlite::memory:").await.unwrap();

    let now = Utc::now();
    let packet_id = Uuid::new_v4();

    let mut points = Vec::new();

    for i in 0..10 {
        let speed = if i == 5 { 500.0 } else { 40.0 };
        points.push(TrackPoint {
            id: None,
            device_id: "speed_filter_001".to_string(),
            latitude: 39.9000 + (i as f64) * 0.001,
            longitude: 116.4000 + (i as f64) * 0.001,
            altitude: None,
            speed: Some(speed),
            heading: Some(45.0),
            satellites: Some(8),
            hdop: None,
            timestamp: now + Duration::seconds(i * 10),
            location_source: Some("GPS".to_string()),
            accuracy: Some(5.0),
            battery_level: Some(85.0),
            extra_data: None,
            distance: None,
            cumulative_distance: None,
            created_at: None,
        });
    }

    db.insert_track_points(&points, packet_id).await.unwrap();

    let query_without_filter = MileageQuery {
        device_id: Some("speed_filter_001".to_string()),
        start_time: None,
        end_time: None,
        max_speed_kmh: None,
        min_accuracy: None,
        stop_timeout_seconds: None,
    };
    let stats_without = db.calculate_mileage(query_without_filter).await.unwrap();
    let points_without = stats_without[0].point_count;

    let query_with_filter = MileageQuery {
        device_id: Some("speed_filter_001".to_string()),
        start_time: None,
        end_time: None,
        max_speed_kmh: Some(200.0),
        min_accuracy: None,
        stop_timeout_seconds: None,
    };
    let stats_with = db.calculate_mileage(query_with_filter).await.unwrap();
    let points_with = stats_with[0].point_count;

    assert!(points_with < points_without, "Speed filter should reduce point count: with={}, without={}", points_with, points_without);
}

#[tokio::test]
async fn test_continuous_mileage_across_packets() {
    use location_tracker::db::Database;
    use location_tracker::models::{SortOrder, TrackPoint, TrackQuery};
    use chrono::{Duration, Utc};
    use uuid::Uuid;

    let db = Database::new("sqlite::memory:").await.unwrap();

    let now = Utc::now();
    let packet_id1 = Uuid::new_v4();
    let packet_id2 = Uuid::new_v4();

    let points1: Vec<TrackPoint> = (0..3)
        .map(|i| TrackPoint {
            id: None,
            device_id: "continuous_001".to_string(),
            latitude: 39.9000 + (i as f64) * 0.001,
            longitude: 116.4000 + (i as f64) * 0.001,
            altitude: None,
            speed: Some(50.0),
            heading: Some(45.0),
            satellites: Some(8),
            hdop: None,
            timestamp: now + Duration::seconds(i * 10),
            location_source: Some("GPS".to_string()),
            accuracy: Some(5.0),
            battery_level: Some(85.0),
            extra_data: None,
            distance: None,
            cumulative_distance: None,
            created_at: None,
        })
        .collect();

    db.insert_track_points(&points1, packet_id1).await.unwrap();

    let points2: Vec<TrackPoint> = (0..3)
        .map(|i| TrackPoint {
            id: None,
            device_id: "continuous_001".to_string(),
            latitude: 39.9030 + (i as f64) * 0.001,
            longitude: 116.4030 + (i as f64) * 0.001,
            altitude: None,
            speed: Some(50.0),
            heading: Some(45.0),
            satellites: Some(8),
            hdop: None,
            timestamp: now + Duration::seconds(30 + i * 10),
            location_source: Some("GPS".to_string()),
            accuracy: Some(5.0),
            battery_level: Some(85.0),
            extra_data: None,
            distance: None,
            cumulative_distance: None,
            created_at: None,
        })
        .collect();

    db.insert_track_points(&points2, packet_id2).await.unwrap();

    let query = TrackQuery {
        device_id: Some("continuous_001".to_string()),
        start_time: None,
        end_time: None,
        limit: None,
        offset: None,
        order: SortOrder::Asc,
    };

    let result = db.get_track_points(query).await.unwrap();
    assert_eq!(result.total, 6);
    assert!(result.total_distance_meters > 500.0 && result.total_distance_meters < 1000.0);

    for i in 1..result.points.len() {
        let cum = result.points[i].cumulative_distance.unwrap();
        let prev_cum = result.points[i - 1].cumulative_distance.unwrap();
        assert!(cum > prev_cum, "Cumulative distance should be continuous across packets at index {}", i);
    }
}

#[test]
fn test_distance_module_haversine() {
    use location_tracker::distance::{haversine_distance_km, haversine_distance_meters, format_distance};

    let d = haversine_distance_meters(39.9042, 116.4074, 39.9042, 116.4074);
    assert!(d < 0.01);

    let d_km = haversine_distance_km(39.9042, 116.4074, 39.9043, 116.4074);
    assert!(d_km > 0.005 && d_km < 0.02);

    assert_eq!(format_distance(500.0), "500.0 m");
    assert!(format_distance(2500.0).contains("km"));
}

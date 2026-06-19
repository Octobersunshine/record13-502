use chrono::Utc;
use reqwest::Client;
use serde_json::{json, Value};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    let base_url = "http://localhost:3000";

    println!("=== Testing Location Tracker API ===\n");

    println!("1. Health Check:");
    let health = client
        .get(format!("{}/health", base_url))
        .send()
        .await?
        .json::<Value>()
        .await?;
    println!("   Response: {}\n", serde_json::to_string_pretty(&health)?);

    println!("2. Upload JSON format location data:");
    let json_payload = json!([
        {
            "latitude": 39.9042,
            "longitude": 116.4074,
            "timestamp": Utc::now().to_rfc3339(),
            "altitude": 45.5,
            "speed": 60.0,
            "heading": 180.0,
            "satellites": 8,
            "source": "GPS",
            "battery": 85.0
        },
        {
            "latitude": 39.9050,
            "longitude": 116.4080,
            "timestamp": (Utc::now() - chrono::Duration::seconds(5)).to_rfc3339(),
            "altitude": 46.0,
            "speed": 62.0,
            "heading": 185.0,
            "satellites": 9,
            "source": "GPS",
            "battery": 84.5
        }
    ]);

    let upload_request = json!({
        "device_id": "dev_001",
        "device_type": "GPS_TRACKER_V2",
        "firmware_version": "1.0.0",
        "upload_time": Utc::now().to_rfc3339(),
        "data_format": "json",
        "payload": serde_json::to_string(&json_payload)?
    });

    let upload_response = client
        .post(format!("{}/api/v1/location/upload", base_url))
        .json(&upload_request)
        .send()
        .await?
        .json::<Value>()
        .await?;
    println!("   Response: {}\n", serde_json::to_string_pretty(&upload_response)?);

    println!("3. Upload Binary format location data:");
    let mut binary_data = Vec::new();
    let now = Utc::now().timestamp() as u32;
    for i in 0..3 {
        let ts = now - i * 5;
        let lat = ((39.9042 + i as f64 * 0.001) * 1_000_000.0) as i32;
        let lng = ((116.4074 + i as f64 * 0.001) * 1_000_000.0) as i32;
        let alt = (45.5 * 10.0) as i16;
        let speed = (60.0 / 0.5) as u8;
        let heading = (180.0 / 1.4117647) as u8;
        let sat = (8 + i) as u8;
        let flags = 0x01u8;

        binary_data.extend_from_slice(&ts.to_le_bytes());
        binary_data.extend_from_slice(&lat.to_le_bytes());
        binary_data.extend_from_slice(&lng.to_le_bytes());
        binary_data.extend_from_slice(&alt.to_le_bytes());
        binary_data.push(speed);
        binary_data.push(heading);
        binary_data.push(sat);
        binary_data.push(flags);
    }

    let binary_payload = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &binary_data);
    let binary_upload_request = json!({
        "device_id": "dev_002",
        "device_type": "GPS_TRACKER_V1",
        "firmware_version": "0.9.5",
        "upload_time": Utc::now().to_rfc3339(),
        "data_format": "binary",
        "payload": binary_payload
    });

    let binary_response = client
        .post(format!("{}/api/v1/location/upload", base_url))
        .json(&binary_upload_request)
        .send()
        .await?
        .json::<Value>()
        .await?;
    println!("   Response: {}\n", serde_json::to_string_pretty(&binary_response)?);

    println!("4. Upload CSV format location data:");
    let csv_data = "latitude,longitude,timestamp,altitude,speed,satellites,source\n\
39.9042,116.4074,2024-01-15T10:25:30Z,45.5,60.0,8,GPS\n\
39.9050,116.4080,2024-01-15T10:25:35Z,46.0,62.0,9,GPS\n\
39.9060,116.4090,2024-01-15T10:25:40Z,47.0,65.0,10,GPS";

    let csv_upload_request = json!({
        "device_id": "dev_003",
        "device_type": "SMARTPHONE_APP",
        "firmware_version": "2.1.0",
        "upload_time": Utc::now().to_rfc3339(),
        "data_format": "csv",
        "payload": csv_data
    });

    let csv_response = client
        .post(format!("{}/api/v1/location/upload", base_url))
        .json(&csv_upload_request)
        .send()
        .await?
        .json::<Value>()
        .await?;
    println!("   Response: {}\n", serde_json::to_string_pretty(&csv_response)?);

    println!("5. Get device list:");
    let devices = client
        .get(format!("{}/api/v1/devices", base_url))
        .send()
        .await?
        .json::<Value>()
        .await?;
    println!("   Response: {}\n", serde_json::to_string_pretty(&devices)?);

    println!("6. Query track points for dev_001:");
    let track_points = client
        .get(format!("{}/api/v1/track/points?device_id=dev_001&limit=10", base_url))
        .send()
        .await?
        .json::<Value>()
        .await?;
    println!("   Response: {}\n", serde_json::to_string_pretty(&track_points)?);

    println!("7. Query all track points:");
    let all_points = client
        .get(format!("{}/api/v1/track/points?limit=50", base_url))
        .send()
        .await?
        .json::<Value>()
        .await?;
    println!("   Total points: {}\n", all_points["total"]);
    println!("   Total distance: {}\n", all_points["total_distance_formatted"]);

    println!("8. Upload with checksum verification:");
    let payload_data = json!([
        {
            "latitude": 40.7128,
            "longitude": -74.0060,
            "timestamp": Utc::now().to_rfc3339(),
            "altitude": 10.0,
            "speed": 45.0,
            "satellites": 12
        }
    ]);
    let payload_str = serde_json::to_string(&payload_data)?;

    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(payload_str.as_bytes());
    let checksum = hex::encode(hasher.finalize());

    let checksum_upload = json!({
        "device_id": "dev_004",
        "upload_time": Utc::now().to_rfc3339(),
        "data_format": "json",
        "payload": payload_str,
        "checksum": checksum
    });

    let checksum_response = client
        .post(format!("{}/api/v1/location/upload", base_url))
        .json(&checksum_upload)
        .send()
        .await?
        .json::<Value>()
        .await?;
    println!("   Response: {}\n", serde_json::to_string_pretty(&checksum_response)?);

    println!("9. Calculate mileage statistics:");
    let mileage_response = client
        .get(format!("{}/api/v1/track/mileage?max_speed_kmh=200&min_accuracy=50", base_url))
        .send()
        .await?
        .json::<Value>()
        .await?;
    println!("   Response: {}\n", serde_json::to_string_pretty(&mileage_response)?);

    println!("10. Calculate mileage for specific device:");
    let device_mileage = client
        .get(format!("{}/api/v1/track/mileage?device_id=dev_001", base_url))
        .send()
        .await?
        .json::<Value>()
        .await?;
    println!("   Response: {}\n", serde_json::to_string_pretty(&device_mileage)?);

    println!("=== All tests completed ===");

    Ok(())
}

use crate::errors::{AppError, AppResult};
use crate::models::{BinaryLocationPoint, DataFormat, LocationDataPacket, TrackPoint};
use bytes::{Buf, Bytes};
use chrono::{DateTime, TimeZone, Utc};
use std::io::Cursor;
use tracing::{debug, info};

pub struct PacketParser;

impl PacketParser {
    pub async fn parse_packet(
        packet: &LocationDataPacket,
        raw_packet_id: uuid::Uuid,
    ) -> AppResult<Vec<TrackPoint>> {
        info!(
            "Parsing packet from device {}, format: {:?}",
            packet.device_id, packet.data_format
        );

        if let Some(checksum) = &packet.checksum {
            Self::verify_checksum(&packet.payload, checksum)?;
        }

        let points = match packet.data_format {
            DataFormat::Json => Self::parse_json(&packet.payload, &packet.device_id)?,
            DataFormat::Binary => {
                let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &packet.payload)?;
                Self::parse_binary(&bytes, &packet.device_id)?
            }
            DataFormat::Hex => {
                let bytes = hex::decode(&packet.payload)?;
                Self::parse_binary(&bytes, &packet.device_id)?
            }
            DataFormat::Csv => {
                let decoded = if Self::is_base64(&packet.payload) {
                    let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &packet.payload)?;
                    String::from_utf8(bytes)?
                } else {
                    packet.payload.clone()
                };
                Self::parse_csv(&decoded, &packet.device_id)?
            }
            DataFormat::Protobuf => {
                let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &packet.payload)?;
                Self::parse_protobuf(&bytes, &packet.device_id)?
            }
        };

        debug!("Parsed {} track points", points.len());

        let mut points: Vec<TrackPoint> = points
            .into_iter()
            .map(|mut p| {
                p.id = Some(uuid::Uuid::new_v4());
                p
            })
            .collect();

        points.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

        debug!("Sorted {} track points by timestamp ascending", points.len());

        Ok(points)
    }

    fn verify_checksum(payload: &str, expected: &str) -> AppResult<()> {
        use sha2::{Digest, Sha256};

        let mut hasher = Sha256::new();
        hasher.update(payload.as_bytes());
        let result = hasher.finalize();
        let actual = hex::encode(result);

        if actual.to_lowercase() != expected.to_lowercase() {
            return Err(AppError::ChecksumError);
        }

        Ok(())
    }

    fn is_base64(s: &str) -> bool {
        base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            s,
        )
        .is_ok()
    }

    fn parse_json(payload: &str, device_id: &str) -> AppResult<Vec<TrackPoint>> {
        let decoded = if Self::is_base64(payload) {
            let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, payload)?;
            String::from_utf8(bytes)?
        } else {
            payload.to_string()
        };

        let json_value: serde_json::Value = serde_json::from_str(&decoded)
            .map_err(|e| AppError::ParseError(format!("Invalid JSON: {}", e)))?;

        let points = if let Some(array) = json_value.as_array() {
            let mut points = Vec::new();
            for item in array {
                points.push(Self::json_to_track_point(item, device_id)?);
            }
            points
        } else if json_value.is_object() {
            vec![Self::json_to_track_point(&json_value, device_id)?]
        } else if let Some(points_field) = json_value.get("points").and_then(|v| v.as_array()) {
            let mut points = Vec::new();
            for item in points_field {
                points.push(Self::json_to_track_point(item, device_id)?);
            }
            points
        } else if let Some(data_field) = json_value.get("data").and_then(|v| v.as_array()) {
            let mut points = Vec::new();
            for item in data_field {
                points.push(Self::json_to_track_point(item, device_id)?);
            }
            points
        } else {
            return Err(AppError::ParseError(
                "JSON must be an array or contain 'points'/'data' field".to_string(),
            ));
        };

        Ok(points)
    }

    fn json_to_track_point(
        json: &serde_json::Value,
        device_id: &str,
    ) -> AppResult<TrackPoint> {
        let latitude = json
            .get("latitude")
            .or_else(|| json.get("lat"))
            .and_then(|v| v.as_f64())
            .ok_or_else(|| AppError::ParseError("Missing or invalid latitude".to_string()))?;

        let longitude = json
            .get("longitude")
            .or_else(|| json.get("lng"))
            .or_else(|| json.get("lon"))
            .and_then(|v| v.as_f64())
            .ok_or_else(|| AppError::ParseError("Missing or invalid longitude".to_string()))?;

        let timestamp_str = json
            .get("timestamp")
            .or_else(|| json.get("time"))
            .or_else(|| json.get("t"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::ParseError("Missing timestamp".to_string()))?;

        let timestamp = DateTime::parse_from_rfc3339(timestamp_str)
            .or_else(|_| {
                timestamp_str
                    .parse::<i64>()
                    .map(|secs| Utc.timestamp_opt(secs, 0).single().unwrap())
                    .map_err(|_| AppError::ParseError(format!("Invalid timestamp: {}", timestamp_str)))
            })?
            .with_timezone(&Utc);

        let point = TrackPoint {
            id: None,
            device_id: device_id.to_string(),
            latitude,
            longitude,
            altitude: json
                .get("altitude")
                .or_else(|| json.get("alt"))
                .and_then(|v| v.as_f64()),
            speed: json
                .get("speed")
                .or_else(|| json.get("spd"))
                .and_then(|v| v.as_f64()),
            heading: json
                .get("heading")
                .or_else(|| json.get("dir"))
                .or_else(|| json.get("bearing"))
                .and_then(|v| v.as_f64()),
            satellites: json
                .get("satellites")
                .or_else(|| json.get("sat"))
                .and_then(|v| v.as_i64().map(|n| n as i32)),
            hdop: json
                .get("hdop")
                .or_else(|| json.get("accuracy"))
                .and_then(|v| v.as_f64()),
            timestamp,
            location_source: json
                .get("source")
                .or_else(|| json.get("provider"))
                .and_then(|v| v.as_str().map(|s| s.to_string())),
            accuracy: json
                .get("accuracy")
                .and_then(|v| v.as_f64()),
            battery_level: json
                .get("battery")
                .or_else(|| json.get("batt"))
                .and_then(|v| v.as_f64().map(|f| f as f32)),
            extra_data: Some(json.clone()),
            created_at: None,
        };

        Ok(point)
    }

    fn parse_binary(data: &[u8], device_id: &str) -> AppResult<Vec<TrackPoint>> {
        const POINT_SIZE: usize = 16;
        let total_points = data.len() / POINT_SIZE;

        if data.len() % POINT_SIZE != 0 {
            return Err(AppError::ParseError(format!(
                "Binary data length {} is not a multiple of {}",
                data.len(),
                POINT_SIZE
            )));
        }

        let mut points = Vec::with_capacity(total_points);
        let mut cursor = Cursor::new(data);

        for i in 0..total_points {
            let binary_point = BinaryLocationPoint {
                timestamp: cursor.get_u32_le(),
                latitude: cursor.get_i32_le(),
                longitude: cursor.get_i32_le(),
                altitude: cursor.get_i16_le(),
                speed: cursor.get_u8(),
                heading: cursor.get_u8(),
                satellites: cursor.get_u8(),
                flags: cursor.get_u8(),
            };

            let timestamp = Utc
                .timestamp_opt(binary_point.timestamp as i64, 0)
                .single()
                .ok_or_else(|| {
                    AppError::ParseError(format!("Invalid timestamp at point {}", i))
                })?;

            let latitude = binary_point.latitude as f64 / 1_000_000.0;
            let longitude = binary_point.longitude as f64 / 1_000_000.0;
            let altitude = binary_point.altitude as f64 / 10.0;
            let speed = binary_point.speed as f64 * 0.5;
            let heading = binary_point.heading as f64 * 1.4117647;

            let source = if binary_point.flags & 0x01 != 0 {
                Some("GPS".to_string())
            } else if binary_point.flags & 0x02 != 0 {
                Some("LBS".to_string())
            } else if binary_point.flags & 0x04 != 0 {
                Some("WIFI".to_string())
            } else {
                None
            };

            let battery = if binary_point.flags & 0x08 != 0 {
                Some((binary_point.speed as f32 % 100.0) / 100.0)
            } else {
                None
            };

            points.push(TrackPoint {
                id: None,
                device_id: device_id.to_string(),
                latitude,
                longitude,
                altitude: Some(altitude),
                speed: Some(speed),
                heading: Some(heading),
                satellites: Some(binary_point.satellites as i32),
                hdop: None,
                timestamp,
                location_source: source,
                accuracy: None,
                battery_level: battery,
                extra_data: Some(serde_json::json!({
                    "flags": binary_point.flags,
                    "raw_lat": binary_point.latitude,
                    "raw_lng": binary_point.longitude,
                })),
                created_at: None,
            });
        }

        Ok(points)
    }

    fn parse_csv(payload: &str, device_id: &str) -> AppResult<Vec<TrackPoint>> {
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(true)
            .flexible(true)
            .from_reader(payload.as_bytes());

        let headers = reader
            .headers()
            .map_err(|e| AppError::ParseError(format!("CSV header error: {}", e)))?
            .clone();

        let lat_idx = headers
            .iter()
            .position(|h| h.eq_ignore_ascii_case("latitude") || h.eq_ignore_ascii_case("lat"));
        let lng_idx = headers
            .iter()
            .position(|h| h.eq_ignore_ascii_case("longitude") || h.eq_ignore_ascii_case("lng") || h.eq_ignore_ascii_case("lon"));
        let time_idx = headers
            .iter()
            .position(|h| h.eq_ignore_ascii_case("timestamp") || h.eq_ignore_ascii_case("time") || h.eq_ignore_ascii_case("t"));
        let alt_idx = headers
            .iter()
            .position(|h| h.eq_ignore_ascii_case("altitude") || h.eq_ignore_ascii_case("alt"));
        let speed_idx = headers
            .iter()
            .position(|h| h.eq_ignore_ascii_case("speed") || h.eq_ignore_ascii_case("spd"));
        let heading_idx = headers
            .iter()
            .position(|h| h.eq_ignore_ascii_case("heading") || h.eq_ignore_ascii_case("dir") || h.eq_ignore_ascii_case("bearing"));
        let sat_idx = headers
            .iter()
            .position(|h| h.eq_ignore_ascii_case("satellites") || h.eq_ignore_ascii_case("sat"));

        let lat_idx = lat_idx.ok_or_else(|| AppError::ParseError("CSV missing latitude column".to_string()))?;
        let lng_idx = lng_idx.ok_or_else(|| AppError::ParseError("CSV missing longitude column".to_string()))?;
        let time_idx = time_idx.ok_or_else(|| AppError::ParseError("CSV missing timestamp column".to_string()))?;

        let mut points = Vec::new();

        for (i, result) in reader.records().enumerate() {
            let record = result
                .map_err(|e| AppError::ParseError(format!("CSV parse error at line {}: {}", i + 2, e)))?;

            let latitude: f64 = record
                .get(lat_idx)
                .and_then(|s| s.parse().ok())
                .ok_or_else(|| AppError::ParseError(format!("Invalid latitude at line {}", i + 2)))?;

            let longitude: f64 = record
                .get(lng_idx)
                .and_then(|s| s.parse().ok())
                .ok_or_else(|| AppError::ParseError(format!("Invalid longitude at line {}", i + 2)))?;

            let timestamp_str = record
                .get(time_idx)
                .ok_or_else(|| AppError::ParseError(format!("Missing timestamp at line {}", i + 2)))?;

            let timestamp = DateTime::parse_from_rfc3339(timestamp_str)
                .or_else(|_| {
                    timestamp_str
                        .parse::<i64>()
                        .map(|secs| Utc.timestamp_opt(secs, 0).single().unwrap())
                        .map_err(|_| AppError::ParseError(format!("Invalid timestamp at line {}: {}", i + 2, timestamp_str)))
                })?
                .with_timezone(&Utc);

            let point = TrackPoint {
                id: None,
                device_id: device_id.to_string(),
                latitude,
                longitude,
                altitude: alt_idx.and_then(|idx| record.get(idx).and_then(|s| s.parse().ok())),
                speed: speed_idx.and_then(|idx| record.get(idx).and_then(|s| s.parse().ok())),
                heading: heading_idx.and_then(|idx| record.get(idx).and_then(|s| s.parse().ok())),
                satellites: sat_idx.and_then(|idx| record.get(idx).and_then(|s| s.parse::<i32>().ok())),
                hdop: None,
                timestamp,
                location_source: None,
                accuracy: None,
                battery_level: None,
                extra_data: Some(serde_json::json!({
                    "row": record.iter().collect::<Vec<_>>(),
                    "headers": headers.iter().collect::<Vec<_>>(),
                })),
                created_at: None,
            };

            points.push(point);
        }

        Ok(points)
    }

    fn parse_protobuf(data: &[u8], device_id: &str) -> AppResult<Vec<TrackPoint>> {
        let mut points = Vec::new();
        let bytes = Bytes::copy_from_slice(data);
        let mut cursor = 0;

        while cursor < bytes.len() {
            if cursor + 1 > bytes.len() {
                break;
            }
            let tag = bytes[cursor];
            cursor += 1;

            let field_number = tag >> 3;
            let wire_type = tag & 0x07;

            match (field_number, wire_type) {
                (1, 2) => {
                    let (len, len_bytes) = Self::decode_varint(&bytes[cursor..])?;
                    cursor += len_bytes;

                    if cursor + len > bytes.len() {
                        return Err(AppError::ParseError("Protobuf message truncated".to_string()));
                    }

                    let point_data = &bytes[cursor..cursor + len];
                    cursor += len;

                    if let Some(point) = Self::parse_protobuf_track_point(point_data, device_id)? {
                        points.push(point);
                    }
                }
                _ => {
                    let (_, skip_bytes) = Self::skip_field(&bytes[cursor..], wire_type)?;
                    cursor += skip_bytes;
                }
            }
        }

        Ok(points)
    }

    fn parse_protobuf_track_point(
        data: &[u8],
        device_id: &str,
    ) -> AppResult<Option<TrackPoint>> {
        let mut latitude: Option<f64> = None;
        let mut longitude: Option<f64> = None;
        let mut timestamp: Option<i64> = None;
        let mut altitude: Option<f64> = None;
        let mut speed: Option<f64> = None;
        let mut heading: Option<f64> = None;
        let mut satellites: Option<i32> = None;

        let mut cursor = 0;
        while cursor < data.len() {
            if cursor + 1 > data.len() {
                break;
            }
            let tag = data[cursor];
            cursor += 1;

            let field_number = tag >> 3;
            let wire_type = tag & 0x07;

            match (field_number, wire_type) {
                (1, 1) => {
                    if cursor + 8 > data.len() {
                        return Err(AppError::ParseError("Protobuf double truncated".to_string()));
                    }
                    let val = f64::from_le_bytes(data[cursor..cursor + 8].try_into().unwrap());
                    latitude = Some(val);
                    cursor += 8;
                }
                (2, 1) => {
                    if cursor + 8 > data.len() {
                        return Err(AppError::ParseError("Protobuf double truncated".to_string()));
                    }
                    let val = f64::from_le_bytes(data[cursor..cursor + 8].try_into().unwrap());
                    longitude = Some(val);
                    cursor += 8;
                }
                (3, 0) => {
                    let (val, len) = Self::decode_varint(&data[cursor..])?;
                    timestamp = Some(val as i64);
                    cursor += len;
                }
                (4, 1) | (4, 5) => {
                    if wire_type == 1 {
                        if cursor + 8 > data.len() {
                            return Err(AppError::ParseError("Protobuf double truncated".to_string()));
                        }
                        let val = f64::from_le_bytes(data[cursor..cursor + 8].try_into().unwrap());
                        altitude = Some(val);
                        cursor += 8;
                    } else {
                        if cursor + 4 > data.len() {
                            return Err(AppError::ParseError("Protobuf float truncated".to_string()));
                        }
                        let val = f32::from_le_bytes(data[cursor..cursor + 4].try_into().unwrap());
                        altitude = Some(val as f64);
                        cursor += 4;
                    }
                }
                (5, 0) => {
                    let (val, len) = Self::decode_varint(&data[cursor..])?;
                    speed = Some(val as f64 * 0.1);
                    cursor += len;
                }
                (6, 0) => {
                    let (val, len) = Self::decode_varint(&data[cursor..])?;
                    heading = Some(val as f64 * 0.1);
                    cursor += len;
                }
                (7, 0) => {
                    let (val, len) = Self::decode_varint(&data[cursor..])?;
                    satellites = Some(val as i32);
                    cursor += len;
                }
                _ => {
                    let (_, skip_bytes) = Self::skip_field(&data[cursor..], wire_type)?;
                    cursor += skip_bytes;
                }
            }
        }

        let lat = latitude.ok_or_else(|| AppError::ParseError("Protobuf missing latitude".to_string()))?;
        let lng = longitude.ok_or_else(|| AppError::ParseError("Protobuf missing longitude".to_string()))?;
        let ts = timestamp.ok_or_else(|| AppError::ParseError("Protobuf missing timestamp".to_string()))?;

        let timestamp = Utc
            .timestamp_opt(ts, 0)
            .single()
            .ok_or_else(|| AppError::ParseError("Invalid protobuf timestamp".to_string()))?;

        Ok(Some(TrackPoint {
            id: None,
            device_id: device_id.to_string(),
            latitude: lat,
            longitude: lng,
            altitude,
            speed,
            heading,
            satellites,
            hdop: None,
            timestamp,
            location_source: None,
            accuracy: None,
            battery_level: None,
            extra_data: None,
            created_at: None,
        }))
    }

    fn decode_varint(data: &[u8]) -> AppResult<(u64, usize)> {
        let mut result: u64 = 0;
        let mut shift: u32 = 0;
        let mut bytes_read = 0;

        for &byte in data.iter() {
            bytes_read += 1;
            result |= ((byte & 0x7F) as u64) << shift;
            if byte & 0x80 == 0 {
                return Ok((result, bytes_read));
            }
            shift += 7;
            if shift >= 64 {
                return Err(AppError::ParseError("Varint too long".to_string()));
            }
        }

        Err(AppError::ParseError("Truncated varint".to_string()))
    }

    fn skip_field(data: &[u8], wire_type: u8) -> AppResult<(u64, usize)> {
        match wire_type {
            0 => Self::decode_varint(data),
            1 => Ok((0, 8)),
            2 => {
                let (len, len_bytes) = Self::decode_varint(data)?;
                Ok((0, len_bytes + len as usize))
            }
            5 => Ok((0, 4)),
            _ => Err(AppError::ParseError(format!("Unknown wire type: {}", wire_type))),
        }
    }
}

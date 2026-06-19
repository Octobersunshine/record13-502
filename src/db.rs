use crate::distance::{calculate_cumulative_distances, calculate_segment_distance, filter_and_calculate_distance, format_distance};
use crate::errors::{AppError, AppResult};
use crate::models::{LocationDataPacket, MileageQuery, MileageStats, RawPacketRecord, SortOrder, TrackPoint, TrackQuery, TrackListResponse};
use chrono::{DateTime, Duration, Utc};
use sqlx::{sqlite::SqlitePool, SqlitePoolOptions};
use tracing::{info, debug, warn};
use uuid::Uuid;

#[derive(Clone)]
pub struct Database {
    pub pool: SqlitePool,
}

impl Database {
    pub async fn new(database_url: &str) -> AppResult<Self> {
        let pool = SqlitePoolOptions::new()
            .max_connections(10)
            .connect(database_url)
            .await?;

        info!("Database connected: {}", database_url);

        let db = Self { pool };
        db.init_tables().await?;

        Ok(db)
    }

    async fn init_tables(&self) -> AppResult<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS raw_packets (
                id TEXT PRIMARY KEY,
                device_id TEXT NOT NULL,
                device_type TEXT,
                firmware_version TEXT,
                upload_time TEXT NOT NULL,
                data_format TEXT NOT NULL,
                payload TEXT NOT NULL,
                checksum TEXT,
                signature TEXT,
                parsed INTEGER DEFAULT 0,
                parse_error TEXT,
                created_at TEXT NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS track_points (
                id TEXT PRIMARY KEY,
                device_id TEXT NOT NULL,
                latitude REAL NOT NULL,
                longitude REAL NOT NULL,
                altitude REAL,
                speed REAL,
                heading REAL,
                satellites INTEGER,
                hdop REAL,
                timestamp TEXT NOT NULL,
                location_source TEXT,
                accuracy REAL,
                battery_level REAL,
                extra_data TEXT,
                distance REAL DEFAULT 0,
                cumulative_distance REAL DEFAULT 0,
                raw_packet_id TEXT,
                created_at TEXT NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        let _ = sqlx::query(
            "ALTER TABLE track_points ADD COLUMN distance REAL DEFAULT 0"
        )
        .execute(&self.pool)
        .await;

        let _ = sqlx::query(
            "ALTER TABLE track_points ADD COLUMN cumulative_distance REAL DEFAULT 0"
        )
        .execute(&self.pool)
        .await;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_track_points_device_time ON track_points(device_id, timestamp)"
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_track_points_timestamp ON track_points(timestamp)"
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_raw_packets_device ON raw_packets(device_id)"
        )
        .execute(&self.pool)
        .await?;

        info!("Database tables initialized");
        Ok(())
    }

    pub async fn insert_raw_packet(
        &self,
        packet: &LocationDataPacket,
    ) -> AppResult<Uuid> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        sqlx::query(
            r#"
            INSERT INTO raw_packets (
                id, device_id, device_type, firmware_version,
                upload_time, data_format, payload,
                checksum, signature, parsed, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 0, ?)
            "#,
        )
        .bind(id.to_string())
        .bind(&packet.device_id)
        .bind(&packet.device_type)
        .bind(&packet.firmware_version)
        .bind(packet.upload_time.to_rfc3339())
        .bind(format!("{:?}", packet.data_format).to_lowercase())
        .bind(&packet.payload)
        .bind(&packet.checksum)
        .bind(&packet.signature)
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await?;

        debug!("Raw packet inserted: {}", id);
        Ok(id)
    }

    pub async fn mark_packet_parsed(
        &self,
        packet_id: Uuid,
        success: bool,
        error: Option<&str>,
    ) -> AppResult<()> {
        sqlx::query(
            r#"
            UPDATE raw_packets
            SET parsed = ?, parse_error = ?
            WHERE id = ?
            "#,
        )
        .bind(if success { 1 } else { 0 })
        .bind(error)
        .bind(packet_id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn insert_track_points(
        &self,
        points: &[TrackPoint],
        raw_packet_id: Uuid,
    ) -> AppResult<usize> {
        if points.is_empty() {
            return Ok(0);
        }

        let device_id = &points[0].device_id;
        let last_prev_point = self.get_last_point_before(device_id, points[0].timestamp).await?;

        let mut cumulative_base = 0.0;
        let mut prev_point = last_prev_point.clone();

        if let Some(ref last) = last_prev_point {
            cumulative_base = last.cumulative_distance.unwrap_or(0.0);
        }

        let mut enhanced_points: Vec<TrackPoint> = Vec::with_capacity(points.len());

        for (idx, point) in points.iter().enumerate() {
            let mut ep = point.clone();

            let distance_from_prev = if idx == 0 {
                if let Some(ref prev) = prev_point {
                    calculate_segment_distance(prev, point)
                } else {
                    0.0
                }
            } else {
                calculate_segment_distance(&points[idx - 1], point)
            };

            ep.distance = Some(distance_from_prev);
            cumulative_base += distance_from_prev;
            ep.cumulative_distance = Some(cumulative_base);

            prev_point = Some(ep.clone());
            enhanced_points.push(ep);
        }

        let mut inserted = 0;
        let raw_packet_id_str = raw_packet_id.to_string();

        for point in &enhanced_points {
            let id = point.id.unwrap_or_else(Uuid::new_v4);
            let now = Utc::now();

            sqlx::query(
                r#"
                INSERT INTO track_points (
                    id, device_id, latitude, longitude, altitude,
                    speed, heading, satellites, hdop, timestamp,
                    location_source, accuracy, battery_level,
                    extra_data, distance, cumulative_distance,
                    raw_packet_id, created_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(id.to_string())
            .bind(&point.device_id)
            .bind(point.latitude)
            .bind(point.longitude)
            .bind(point.altitude)
            .bind(point.speed)
            .bind(point.heading)
            .bind(point.satellites)
            .bind(point.hdop)
            .bind(point.timestamp.to_rfc3339())
            .bind(&point.location_source)
            .bind(point.accuracy)
            .bind(point.battery_level)
            .bind(point.extra_data.as_ref().map(|v| v.to_string()))
            .bind(point.distance)
            .bind(point.cumulative_distance)
            .bind(&raw_packet_id_str)
            .bind(now.to_rfc3339())
            .execute(&self.pool)
            .await?;

            inserted += 1;
        }

        debug!(
            "Inserted {} track points with distance calculation for device {}",
            inserted, device_id
        );
        Ok(inserted)
    }

    async fn get_last_point_before(
        &self,
        device_id: &str,
        before_time: DateTime<Utc>,
    ) -> AppResult<Option<TrackPoint>> {
        let row: Option<(
            String,
            String,
            f64,
            f64,
            Option<f64>,
            Option<f64>,
            Option<f64>,
            Option<i64>,
            Option<f64>,
            String,
            Option<String>,
            Option<f64>,
            Option<f64>,
            Option<String>,
            Option<f64>,
            Option<f64>,
            String,
        )> = sqlx::query_as(
            r#"
            SELECT id, device_id, latitude, longitude, altitude, speed, heading, satellites, hdop,
                   timestamp, location_source, accuracy, battery_level, extra_data,
                   distance, cumulative_distance, created_at
            FROM track_points
            WHERE device_id = ? AND timestamp < ?
            ORDER BY timestamp DESC
            LIMIT 1
            "#,
        )
        .bind(device_id)
        .bind(before_time.to_rfc3339())
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|row| {
            let (
                id,
                device_id,
                latitude,
                longitude,
                altitude,
                speed,
                heading,
                satellites,
                hdop,
                timestamp,
                location_source,
                accuracy,
                battery_level,
                extra_data,
                distance,
                cumulative_distance,
                created_at,
            ) = row;

            TrackPoint {
                id: Some(Uuid::parse_str(&id).unwrap()),
                device_id,
                latitude,
                longitude,
                altitude,
                speed,
                heading,
                satellites: satellites.map(|s| s as i32),
                hdop,
                timestamp: DateTime::parse_from_rfc3339(&timestamp).unwrap().with_timezone(&Utc),
                location_source,
                accuracy,
                battery_level: battery_level.map(|b| b as f32),
                extra_data: extra_data.as_deref().map(|s| serde_json::from_str(s).ok()),
                distance,
                cumulative_distance,
                created_at: Some(DateTime::parse_from_rfc3339(&created_at).unwrap().with_timezone(&Utc)),
            }
        }))
    }

    pub async fn get_track_points(
        &self,
        query: TrackQuery,
    ) -> AppResult<TrackListResponse> {
        let mut sql = "SELECT id, device_id, latitude, longitude, altitude, speed, heading, satellites, hdop, timestamp, location_source, accuracy, battery_level, extra_data, distance, cumulative_distance, created_at FROM track_points WHERE 1=1";
        let mut count_sql = "SELECT COUNT(*) FROM track_points WHERE 1=1";
        let mut conditions: Vec<String> = Vec::new();
        let mut params: Vec<String> = Vec::new();

        if let Some(device_id) = &query.device_id {
            conditions.push("device_id = ?".to_string());
            params.push(device_id.clone());
        }

        if let Some(start_time) = &query.start_time {
            conditions.push("timestamp >= ?".to_string());
            params.push(start_time.to_rfc3339());
        }

        if let Some(end_time) = &query.end_time {
            conditions.push("timestamp <= ?".to_string());
            params.push(end_time.to_rfc3339());
        }

        if !conditions.is_empty() {
            let cond_str = conditions.join(" AND ");
            sql = &format!("{} AND {}", sql, cond_str);
            count_sql = &format!("{} AND {}", count_sql, cond_str);
        }

        let order_str = match query.order {
            SortOrder::Asc => "ORDER BY timestamp ASC",
            SortOrder::Desc => "ORDER BY timestamp DESC",
        };
        sql = &format!("{} {}", sql, order_str);

        if let Some(limit) = query.limit {
            sql = &format!("{} LIMIT {}", sql, limit);
        } else {
            sql = &format!("{} LIMIT 100", sql);
        }

        if let Some(offset) = query.offset {
            sql = &format!("{} OFFSET {}", sql, offset);
        }

        let count: i64 = {
            let mut q = sqlx::query_scalar(count_sql);
            for p in &params {
                q = q.bind(p);
            }
            q.fetch_one(&self.pool).await?
        };

        let rows: Vec<(
            String,
            String,
            f64,
            f64,
            Option<f64>,
            Option<f64>,
            Option<f64>,
            Option<i64>,
            Option<f64>,
            String,
            Option<String>,
            Option<f64>,
            Option<f64>,
            Option<String>,
            Option<f64>,
            Option<f64>,
            String,
        )> = {
            let mut q = sqlx::query_as(sql);
            for p in &params {
                q = q.bind(p);
            }
            q.fetch_all(&self.pool).await?
        };

        let points: Vec<TrackPoint> = rows
            .into_iter()
            .map(
                |row| -> Result<TrackPoint, AppError> {
                    let (
                        id,
                        device_id,
                        latitude,
                        longitude,
                        altitude,
                        speed,
                        heading,
                        satellites,
                        hdop,
                        timestamp,
                        location_source,
                        accuracy,
                        battery_level,
                        extra_data,
                        distance,
                        cumulative_distance,
                        created_at,
                    ) = row;

                    Ok(TrackPoint {
                        id: Some(Uuid::parse_str(&id)?),
                        device_id,
                        latitude,
                        longitude,
                        altitude,
                        speed,
                        heading,
                        satellites: satellites.map(|s| s as i32),
                        hdop,
                        timestamp: DateTime::parse_from_rfc3339(&timestamp)?.with_timezone(&Utc),
                        location_source,
                        accuracy,
                        battery_level: battery_level.map(|b| b as f32),
                        extra_data: extra_data
                            .as_deref()
                            .map(|s| serde_json::from_str(s).ok()),
                        distance,
                        cumulative_distance,
                        created_at: Some(
                            DateTime::parse_from_rfc3339(&created_at)?.with_timezone(&Utc),
                    })
                },
            )
            .collect::<Result<Vec<_>, _>>()?;

        let total_distance_meters: f64 = if query.order == SortOrder::Asc {
            calculate_total_distance(&points)
        } else {
            let mut reversed = points.clone();
            reversed.reverse();
            calculate_total_distance(&reversed)
        };

        Ok(TrackListResponse {
            total: count,
            points,
            total_distance_meters,
            total_distance_formatted: format_distance(total_distance_meters),
        })
    }

    pub async fn get_track_point_by_id(&self, id: Uuid) -> AppResult<Option<TrackPoint>> {
        let row: Option<(
            String,
            String,
            f64,
            f64,
            Option<f64>,
            Option<f64>,
            Option<f64>,
            Option<i64>,
            Option<f64>,
            String,
            Option<String>,
            Option<f64>,
            Option<f64>,
            Option<String>,
            Option<f64>,
            Option<f64>,
            String,
        )> = sqlx::query_as(
            r#"
            SELECT id, device_id, latitude, longitude, altitude, speed, heading, satellites, hdop, timestamp, location_source, accuracy, battery_level, extra_data, distance, cumulative_distance, created_at
            FROM track_points WHERE id = ?
            "#,
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|row| {
            let (
                id,
                device_id,
                latitude,
                longitude,
                altitude,
                speed,
                heading,
                satellites,
                hdop,
                timestamp,
                location_source,
                accuracy,
                battery_level,
                extra_data,
                distance,
                cumulative_distance,
                created_at,
            ) = row;

            TrackPoint {
                id: Some(Uuid::parse_str(&id).unwrap()),
                device_id,
                latitude,
                longitude,
                altitude,
                speed,
                heading,
                satellites: satellites.map(|s| s as i32),
                hdop,
                timestamp: DateTime::parse_from_rfc3339(&timestamp).unwrap().with_timezone(&Utc),
                location_source,
                accuracy,
                battery_level: battery_level.map(|b| b as f32),
                extra_data: extra_data.as_deref().map(|s| serde_json::from_str(s).ok()),
                distance,
                cumulative_distance,
                created_at: Some(DateTime::parse_from_rfc3339(&created_at).unwrap().with_timezone(&Utc)),
            }
        }))
    }

    pub async fn get_raw_packet(&self, id: Uuid) -> AppResult<Option<RawPacketRecord>> {
        let row: Option<(
            String,
            String,
            Option<String>,
            Option<String>,
            String,
            String,
            String,
            Option<String>,
            Option<String>,
            i64,
            Option<String>,
            String,
        )> = sqlx::query_as(
            r#"
            SELECT id, device_id, device_type, firmware_version, upload_time, data_format, payload, checksum, signature, parsed, parse_error, created_at
            FROM raw_packets WHERE id = ?
            "#,
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|row| {
            let (
                id,
                device_id,
                device_type,
                firmware_version,
                upload_time,
                data_format,
                payload,
                checksum,
                signature,
                parsed,
                parse_error,
                created_at,
            ) = row;

            RawPacketRecord {
                id: Uuid::parse_str(&id).unwrap(),
                device_id,
                device_type,
                firmware_version,
                upload_time: DateTime::parse_from_rfc3339(&upload_time).unwrap().with_timezone(&Utc),
                data_format,
                payload,
                checksum,
                signature,
                parsed: parsed != 0,
                parse_error,
                created_at: DateTime::parse_from_rfc3339(&created_at).unwrap().with_timezone(&Utc),
            }
        }))
    }

    pub async fn delete_track_point(&self, id: Uuid) -> AppResult<bool> {
        let result = sqlx::query("DELETE FROM track_points WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn get_device_list(&self) -> AppResult<Vec<String>> {
        let devices: Vec<(String,)> = sqlx::query_as(
            "SELECT DISTINCT device_id FROM track_points ORDER BY device_id"
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(devices.into_iter().map(|d| d.0).collect())
    }

    pub async fn calculate_mileage(
        &self,
        query: MileageQuery,
    ) -> AppResult<Vec<MileageStats>> {
        debug!("Calculating mileage with query: {:?}", query);

        let mut sql = "SELECT id, device_id, latitude, longitude, altitude, speed, heading, satellites, hdop, timestamp, location_source, accuracy, battery_level, extra_data, distance, cumulative_distance, created_at FROM track_points WHERE 1=1";
        let mut conditions: Vec<String> = Vec::new();
        let mut params: Vec<String> = Vec::new();

        if let Some(device_id) = &query.device_id {
            conditions.push("device_id = ?".to_string());
            params.push(device_id.clone());
        }

        if let Some(start_time) = &query.start_time {
            conditions.push("timestamp >= ?".to_string());
            params.push(start_time.to_rfc3339());
        }

        if let Some(end_time) = &query.end_time {
            conditions.push("timestamp <= ?".to_string());
            params.push(end_time.to_rfc3339());
        }

        if !conditions.is_empty() {
            let cond_str = conditions.join(" AND ");
            sql = &format!("{} AND {}", sql, cond_str);
        }

        sql = &format!("{} ORDER BY device_id ASC, timestamp ASC", sql);

        let rows: Vec<(
            String,
            String,
            f64,
            f64,
            Option<f64>,
            Option<f64>,
            Option<f64>,
            Option<i64>,
            Option<f64>,
            String,
            Option<String>,
            Option<f64>,
            Option<f64>,
            Option<String>,
            Option<f64>,
            Option<f64>,
            String,
        )> = {
            let mut q = sqlx::query_as(sql);
            for p in &params {
                q = q.bind(p);
            }
            q.fetch_all(&self.pool).await?
        };

        let mut all_points: Vec<TrackPoint> = Vec::new();
        for row in rows {
            let (
                id,
                device_id,
                latitude,
                longitude,
                altitude,
                speed,
                heading,
                satellites,
                hdop,
                timestamp,
                location_source,
                accuracy,
                battery_level,
                extra_data,
                distance,
                cumulative_distance,
                created_at,
            ) = row;

            all_points.push(TrackPoint {
                id: Some(Uuid::parse_str(&id)?),
                device_id,
                latitude,
                longitude,
                altitude,
                speed,
                heading,
                satellites: satellites.map(|s| s as i32),
                hdop,
                timestamp: DateTime::parse_from_rfc3339(&timestamp)?.with_timezone(&Utc),
                location_source,
                accuracy,
                battery_level: battery_level.map(|b| b as f32),
                extra_data: extra_data.as_deref().map(|s| serde_json::from_str(s).ok()),
                distance,
                cumulative_distance,
                created_at: Some(DateTime::parse_from_rfc3339(&created_at)?.with_timezone(&Utc)),
            });
        }

        let stop_timeout = query.stop_timeout_seconds.unwrap_or(300);

        use std::collections::BTreeMap;
        let mut device_groups: BTreeMap<String, Vec<TrackPoint>> = BTreeMap::new();
        for point in all_points {
            device_groups
                .entry(point.device_id.clone())
                .or_default()
                .push(point);
        }

        let mut results: Vec<MileageStats> = Vec::new();

        for (device_id, points) in device_groups {
            let (filtered_points, total_meters) = filter_and_calculate_distance(
                &points,
                query.max_speed_kmh,
                query.min_accuracy,
            );

            let point_count = filtered_points.len() as i64;
            let start_time = filtered_points.first().map(|p| p.timestamp);
            let end_time = filtered_points.last().map(|p| p.timestamp);

            let mut total_moving_duration = Duration::zero();
            let mut max_speed: Option<f64> = None;
            let mut sum_speed = 0.0;
            let mut speed_count = 0;

            for i in 1..filtered_points.len() {
                let prev = &filtered_points[i - 1];
                let curr = &filtered_points[i];

                let time_diff = (curr.timestamp - prev.timestamp).num_seconds();

                if time_diff < stop_timeout && time_diff > 0 {
                    total_moving_duration = total_moving_duration + Duration::seconds(time_diff);

                    if let Some(speed) = curr.speed {
                        sum_speed += speed;
                        speed_count += 1;
                        if max_speed.is_none() || speed > max_speed.unwrap() {
                            max_speed = Some(speed);
                        }
                    } else if let Some(speed) = prev.speed {
                        sum_speed += speed;
                        speed_count += 1;
                        if max_speed.is_none() || speed > max_speed.unwrap() {
                            max_speed = Some(speed);
                        }
                    }
                }
            }

            let avg_speed_kmh = if speed_count > 0 {
                Some(sum_speed / speed_count as f64)
            } else if total_moving_duration.num_seconds() > 0 && total_meters > 0.0 {
                let hours = total_moving_duration.num_seconds() as f64 / 3600.0;
                Some((total_meters / 1000.0) / hours)
            } else {
                None
            };

            let moving_duration_minutes = if total_moving_duration.num_seconds() > 0 {
                Some(total_moving_duration.num_seconds() as f64 / 60.0)
            } else {
                None
            };

            results.push(MileageStats {
                device_id,
                point_count,
                total_distance_meters: total_meters,
                total_distance_formatted: format_distance(total_meters),
                start_time,
                end_time,
                avg_speed_kmh,
                max_speed_kmh: max_speed,
                moving_duration_minutes,
            });
        }

        info!("Calculated mileage for {} devices", results.len());
        Ok(results)
    }
}

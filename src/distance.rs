use crate::models::TrackPoint;

const EARTH_RADIUS_KM: f64 = 6371.0088;
const EARTH_RADIUS_M: f64 = 6371008.8;

fn to_radians(degrees: f64) -> f64 {
    degrees * std::f64::consts::PI / 180.0
}

pub fn haversine_distance_meters(lat1: f64, lng1: f64, lat2: f64, lng2: f64) -> f64 {
    let dlat = to_radians(lat2 - lat1);
    let dlng = to_radians(lng2 - lng1);

    let lat1_rad = to_radians(lat1);
    let lat2_rad = to_radians(lat2);

    let a = (dlat / 2.0).sin().powi(2)
        + lat1_rad.cos() * lat2_rad.cos() * (dlng / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());

    EARTH_RADIUS_M * c
}

pub fn haversine_distance_km(lat1: f64, lng1: f64, lat2: f64, lng2: f64) -> f64 {
    haversine_distance_meters(lat1, lng1, lat2, lng2) / 1000.0
}

pub fn calculate_segment_distance(prev: &TrackPoint, curr: &TrackPoint) -> f64 {
    haversine_distance_meters(prev.latitude, prev.longitude, curr.latitude, curr.longitude)
}

pub fn calculate_total_distance(points: &[TrackPoint]) -> f64 {
    if points.len() < 2 {
        return 0.0;
    }

    points
        .windows(2)
        .map(|pair| calculate_segment_distance(&pair[0], &pair[1]))
        .sum()
}

pub fn calculate_cumulative_distances(points: &[TrackPoint]) -> Vec<f64> {
    let mut cumulative = Vec::with_capacity(points.len());
    let mut total = 0.0;
    cumulative.push(0.0);

    for i in 1..points.len() {
        total += calculate_segment_distance(&points[i - 1], &points[i]);
        cumulative.push(total);
    }

    cumulative
}

pub fn is_point_valid_for_distance(point: &TrackPoint, min_accuracy: Option<f64>) -> bool {
    if point.latitude < -90.0 || point.latitude > 90.0 {
        return false;
    }
    if point.longitude < -180.0 || point.longitude > 180.0 {
        return false;
    }
    if let Some(acc) = min_accuracy {
        if let Some(point_acc) = point.accuracy {
            if point_acc > acc {
                return false;
            }
        }
    }
    true
}

pub fn filter_and_calculate_distance(
    points: &[TrackPoint],
    max_speed_kmh: Option<f64>,
    min_accuracy: Option<f64>,
) -> (Vec<TrackPoint>, f64) {
    if points.is_empty() {
        return (Vec::new(), 0.0);
    }

    let mut filtered: Vec<TrackPoint> = Vec::new();
    let mut total_distance = 0.0;

    for point in points {
        if !is_point_valid_for_distance(point, min_accuracy) {
            continue;
        }

        if let Some(prev) = filtered.last() {
            let segment_m = calculate_segment_distance(prev, point);
            let time_diff_sec = (point.timestamp - prev.timestamp).num_seconds() as f64;

            if time_diff_sec > 0.0 {
                let speed_kmh = (segment_m / 1000.0) / (time_diff_sec / 3600.0);

                if let Some(max_speed) = max_speed_kmh {
                    if speed_kmh > max_speed {
                        continue;
                    }
                }
            }

            total_distance += segment_m;
        }

        filtered.push(point.clone());
    }

    (filtered, total_distance)
}

pub fn format_distance(meters: f64) -> String {
    if meters < 1000.0 {
        format!("{:.1} m", meters)
    } else {
        format!("{:.2} km", meters / 1000.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_point(lat: f64, lng: f64) -> TrackPoint {
        TrackPoint {
            id: None,
            device_id: "test".to_string(),
            latitude: lat,
            longitude: lng,
            altitude: None,
            speed: None,
            heading: None,
            satellites: None,
            hdop: None,
            timestamp: Utc::now(),
            location_source: None,
            accuracy: None,
            battery_level: None,
            extra_data: None,
            created_at: None,
        }
    }

    #[test]
    fn test_same_point_distance_zero() {
        let d = haversine_distance_meters(39.9042, 116.4074, 39.9042, 116.4074);
        assert!(d < 0.01);
    }

    #[test]
    fn test_known_distance() {
        let beijing = (39.9042, 116.4074);
        let shanghai = (31.2304, 121.4737);
        let dist_km = haversine_distance_km(beijing.0, beijing.1, shanghai.0, shanghai.1);
        assert!((dist_km - 1068.0).abs() < 50.0);
    }

    #[test]
    fn test_small_distance_meters() {
        let p1 = make_point(39.9042, 116.4074);
        let p2 = make_point(39.9042 + 0.001, 116.4074 + 0.001);
        let d = calculate_segment_distance(&p1, &p2);
        assert!(d > 100.0 && d < 200.0);
    }

    #[test]
    fn test_total_distance() {
        let start = 39.9000;
        let points: Vec<TrackPoint> = (0..5)
            .map(|i| make_point(start + (i as f64) * 0.001, 116.4074))
            .collect();

        let total = calculate_total_distance(&points);
        assert!(total > 400.0 && total < 500.0);
    }

    #[test]
    fn test_cumulative_distances() {
        let start = 39.9000;
        let points: Vec<TrackPoint> = (0..4)
            .map(|i| make_point(start + (i as f64) * 0.001, 116.4074))
            .collect();

        let cumulative = calculate_cumulative_distances(&points);
        assert_eq!(cumulative.len(), 4);
        assert_eq!(cumulative[0], 0.0);
        assert!(cumulative[1] > 100.0);
        assert!(cumulative[2] > cumulative[1]);
        assert!(cumulative[3] > cumulative[2]);
    }

    #[test]
    fn test_single_point_zero_distance() {
        let points = vec![make_point(39.9042, 116.4074)];
        assert_eq!(calculate_total_distance(&points), 0.0);
    }

    #[test]
    fn test_empty_points_zero_distance() {
        let points: Vec<TrackPoint> = vec![];
        assert_eq!(calculate_total_distance(&points), 0.0);
    }

    #[test]
    fn test_format_distance() {
        assert_eq!(format_distance(500.0), "500.0 m");
        assert_eq!(format_distance(1500.0), "1.50 km");
        assert_eq!(format_distance(12345.6), "12.35 km");
    }

    #[test]
    fn test_invalid_coordinates() {
        let bad_lat = make_point(95.0, 116.0);
        assert!(!is_point_valid_for_distance(&bad_lat, None));

        let bad_lng = make_point(39.0, 185.0);
        assert!(!is_point_valid_for_distance(&bad_lng, None));

        let good = make_point(39.0, 116.0);
        assert!(is_point_valid_for_distance(&good, None));
    }

    #[test]
    fn test_accuracy_filter() {
        let bad_acc = TrackPoint {
            accuracy: Some(100.0),
            ..make_point(39.0, 116.0)
        };
        assert!(!is_point_valid_for_distance(&bad_acc, Some(50.0)));

        let good_acc = TrackPoint {
            accuracy: Some(10.0),
            ..make_point(39.0, 116.0)
        };
        assert!(is_point_valid_for_distance(&good_acc, Some(50.0)));
    }
}

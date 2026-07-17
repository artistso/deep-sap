//! Bearing geometry and bearing-only localization.
//! Navigation convention: 0° = north, 90° = east.

use std::f64::consts::PI;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bearing {
    pub deg: f64,
    pub radians: f64,
}

impl Bearing {
    pub fn from_deg(deg: f64) -> Self {
        let normalized = deg.rem_euclid(360.0);
        Self {
            deg: normalized,
            radians: normalized.to_radians(),
        }
    }

    pub fn from_rad(rad: f64) -> Self {
        Self::from_deg(rad.to_degrees())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Position2D {
    pub x: f64,
    pub y: f64,
}

impl Position2D {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn zero() -> Self {
        Self::default()
    }

    pub fn distance_to(&self, other: &Position2D) -> f64 {
        (self.x - other.x).hypot(self.y - other.y)
    }

    pub fn bearing_to(&self, other: &Position2D) -> Bearing {
        vector_to_bearing(other.x - self.x, other.y - self.y)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct BearingLineObservation {
    pub observer: Position2D,
    pub bearing: Bearing,
    pub stddev_rad: f64,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Covariance2D {
    pub xx: f64,
    pub xy: f64,
    pub yy: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct FixEstimate {
    pub position: Position2D,
    pub covariance: Covariance2D,
    pub rms_angular_residual_rad: f64,
    pub observations_used: usize,
}

pub fn bearing_to_vector(bearing: Bearing) -> (f64, f64) {
    (bearing.radians.sin(), bearing.radians.cos())
}

pub fn vector_to_bearing(dx: f64, dy: f64) -> Bearing {
    Bearing::from_rad(dx.atan2(dy))
}

pub fn angular_distance_deg(a: f64, b: f64) -> f64 {
    (a - b + 180.0).rem_euclid(360.0) - 180.0
}

pub fn angular_distance_abs_deg(a: f64, b: f64) -> f64 {
    angular_distance_deg(a, b).abs()
}

pub fn haversine_distance_km(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let earth_radius_km = 6371.0;
    let dlat = (lat2 - lat1).to_radians();
    let dlon = (lon2 - lon1).to_radians();
    let a = (dlat / 2.0).sin().powi(2)
        + lat1.to_radians().cos()
            * lat2.to_radians().cos()
            * (dlon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
    earth_radius_km * c
}

pub fn law_of_cosines_distance(a: f64, b: f64, included_angle_deg: f64) -> f64 {
    let c_rad = included_angle_deg.to_radians();
    (a * a + b * b - 2.0 * a * b * c_rad.cos())
        .max(0.0)
        .sqrt()
}

/// Intersect two forward bearing rays. Returns `None` for near-parallel rays
/// and for intersections behind either observer.
pub fn cross_fix_2_bearings(
    p1: Position2D,
    b1: Bearing,
    p2: Position2D,
    b2: Bearing,
) -> Option<Position2D> {
    let (v1x, v1y) = bearing_to_vector(b1);
    let (v2x, v2y) = bearing_to_vector(b2);
    let denom = v1x * v2y - v1y * v2x;

    // About 0.1° of angular separation. Shallower intersections are too
    // ill-conditioned to treat as a useful fix.
    if denom.abs() < 0.001_745 {
        return None;
    }

    let dx = p2.x - p1.x;
    let dy = p2.y - p1.y;
    let t1 = (dx * v2y - dy * v2x) / denom;
    let t2 = (dx * v1y - dy * v1x) / denom;

    if t1 < 0.0 || t2 < 0.0 {
        return None;
    }

    Some(Position2D {
        x: p1.x + t1 * v1x,
        y: p1.y + t1 * v1y,
    })
}

/// Robust initial estimate from pairwise forward-ray intersections.
pub fn trig_triangulation(observations: &[(Position2D, Bearing)]) -> Option<Position2D> {
    if observations.len() < 2 {
        return None;
    }

    let mut fixes = Vec::new();
    for i in 0..observations.len() {
        for j in (i + 1)..observations.len() {
            if let Some(fix) = cross_fix_2_bearings(
                observations[i].0,
                observations[i].1,
                observations[j].0,
                observations[j].1,
            ) {
                if fix.x.is_finite() && fix.y.is_finite() && fix.distance_to(&Position2D::zero()) < 500.0 {
                    fixes.push(fix);
                }
            }
        }
    }

    if fixes.is_empty() {
        return None;
    }

    let mut xs: Vec<f64> = fixes.iter().map(|fix| fix.x).collect();
    let mut ys: Vec<f64> = fixes.iter().map(|fix| fix.y).collect();
    xs.sort_by(f64::total_cmp);
    ys.sort_by(f64::total_cmp);
    let middle = fixes.len() / 2;

    Some(Position2D::new(xs[middle], ys[middle]))
}

/// Nonlinear weighted least-squares bearing fix using Gauss-Newton.
///
/// This estimates a single approximately static position from multiple noisy
/// bearings. Callers must provide measurements from spatially separated sensor
/// poses. The returned covariance is the inverse Fisher information scaled by
/// the measured residual variance.
pub fn bearing_least_squares_fix(
    observations: &[BearingLineObservation],
) -> Option<FixEstimate> {
    if observations.len() < 2 || maximum_observer_baseline_km(observations) < 0.05 {
        return None;
    }

    let initial_lines: Vec<(Position2D, Bearing)> = observations
        .iter()
        .map(|observation| (observation.observer, observation.bearing))
        .collect();
    let mut estimate = trig_triangulation(&initial_lines)?;

    for _ in 0..12 {
        let (a00, a01, a11, b0, b1, _) = information_terms(observations, estimate)?;
        let determinant = a00 * a11 - a01 * a01;
        if determinant.abs() < 1.0e-12 {
            return None;
        }

        let delta_x = (a11 * b0 - a01 * b1) / determinant;
        let delta_y = (-a01 * b0 + a00 * b1) / determinant;
        estimate.x += delta_x;
        estimate.y += delta_y;

        if delta_x.hypot(delta_y) < 1.0e-7 {
            break;
        }
    }

    let (a00, a01, a11, _, _, weighted_residual_sum) =
        information_terms(observations, estimate)?;
    let determinant = a00 * a11 - a01 * a01;
    if determinant.abs() < 1.0e-12 {
        return None;
    }

    let forward_count = observations
        .iter()
        .filter(|observation| {
            let (vx, vy) = bearing_to_vector(observation.bearing);
            let dx = estimate.x - observation.observer.x;
            let dy = estimate.y - observation.observer.y;
            dx * vx + dy * vy > 0.0
        })
        .count();
    if forward_count * 5 < observations.len() * 4 {
        return None;
    }

    let degrees_of_freedom = observations.len().saturating_sub(2).max(1) as f64;
    let residual_scale = (weighted_residual_sum / degrees_of_freedom).max(1.0);
    let covariance = Covariance2D {
        xx: (a11 / determinant) * residual_scale,
        xy: (-a01 / determinant) * residual_scale,
        yy: (a00 / determinant) * residual_scale,
    };

    let rms = angular_rms_residual(observations, estimate);
    Some(FixEstimate {
        position: estimate,
        covariance,
        rms_angular_residual_rad: rms,
        observations_used: observations.len(),
    })
}

fn maximum_observer_baseline_km(observations: &[BearingLineObservation]) -> f64 {
    let mut maximum: f64 = 0.0;
    for i in 0..observations.len() {
        for j in (i + 1)..observations.len() {
            maximum = maximum.max(
                observations[i]
                    .observer
                    .distance_to(&observations[j].observer),
            );
        }
    }
    maximum
}

fn information_terms(
    observations: &[BearingLineObservation],
    estimate: Position2D,
) -> Option<(f64, f64, f64, f64, f64, f64)> {
    let mut a00 = 0.0;
    let mut a01 = 0.0;
    let mut a11 = 0.0;
    let mut b0 = 0.0;
    let mut b1 = 0.0;
    let mut weighted_residual_sum = 0.0;

    for observation in observations {
        let dx = estimate.x - observation.observer.x;
        let dy = estimate.y - observation.observer.y;
        let range_squared = dx * dx + dy * dy;
        if range_squared < 1.0e-8 {
            return None;
        }

        let predicted = dx.atan2(dy);
        let residual = wrap_radians(observation.bearing.radians - predicted);
        let sigma = observation.stddev_rad.clamp(0.25_f64.to_radians(), 15.0_f64.to_radians());
        let weight = 1.0 / (sigma * sigma);

        let jacobian_x = dy / range_squared;
        let jacobian_y = -dx / range_squared;
        a00 += weight * jacobian_x * jacobian_x;
        a01 += weight * jacobian_x * jacobian_y;
        a11 += weight * jacobian_y * jacobian_y;
        b0 += weight * jacobian_x * residual;
        b1 += weight * jacobian_y * residual;
        weighted_residual_sum += weight * residual * residual;
    }

    Some((a00, a01, a11, b0, b1, weighted_residual_sum))
}

fn angular_rms_residual(
    observations: &[BearingLineObservation],
    estimate: Position2D,
) -> f64 {
    let sum_squared = observations
        .iter()
        .map(|observation| {
            let predicted = (estimate.x - observation.observer.x)
                .atan2(estimate.y - observation.observer.y);
            wrap_radians(observation.bearing.radians - predicted).powi(2)
        })
        .sum::<f64>();
    (sum_squared / observations.len().max(1) as f64).sqrt()
}

fn wrap_radians(value: f64) -> f64 {
    (value + PI).rem_euclid(2.0 * PI) - PI
}

/// Bearing-rate range approximation. This is only valid under the standard
/// constant-course, known-ownship-motion assumptions.
pub fn tma_bearing_only_tracking(
    ownship_speed_kts: f64,
    ownship_course: Bearing,
    target_bearing: Bearing,
    bearing_rate_deg_per_sec: f64,
) -> Option<f64> {
    let relative = angular_distance_deg(target_bearing.deg, ownship_course.deg).to_radians();
    let rate_rad_per_second = bearing_rate_deg_per_sec.to_radians();
    if rate_rad_per_second.abs() < 1.0e-8 {
        return None;
    }

    let ownship_speed_km_per_second = ownship_speed_kts * 1.852 / 3600.0;
    let range_km =
        (ownship_speed_km_per_second * relative.sin()).abs() / rate_rad_per_second.abs();
    Some(range_km.clamp(0.05, 500.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn circular_angle_distance_wraps_north() {
        assert!((angular_distance_abs_deg(359.0, 1.0) - 2.0).abs() < 1.0e-9);
    }

    #[test]
    fn cross_fix_requires_both_forward_rays() {
        let p1 = Position2D::new(0.0, 0.0);
        let p2 = Position2D::new(10.0, 0.0);
        let valid = cross_fix_2_bearings(
            p1,
            Bearing::from_deg(45.0),
            p2,
            Bearing::from_deg(315.0),
        )
        .expect("valid forward intersection");
        assert!((valid.x - 5.0).abs() < 0.1);
        assert!((valid.y - 5.0).abs() < 0.1);

        assert!(cross_fix_2_bearings(
            p1,
            Bearing::from_deg(225.0),
            p2,
            Bearing::from_deg(315.0),
        )
        .is_none());
    }

    #[test]
    fn least_squares_recovers_noisy_fix() {
        let target = Position2D::new(8.0, 12.0);
        let observers = [
            Position2D::new(0.0, 0.0),
            Position2D::new(0.4, 0.0),
            Position2D::new(0.8, 0.1),
            Position2D::new(1.2, 0.2),
        ];
        let noise = [0.2, -0.15, 0.1, -0.05];
        let observations: Vec<BearingLineObservation> = observers
            .iter()
            .zip(noise)
            .map(|(observer, noise_deg)| BearingLineObservation {
                observer: *observer,
                bearing: Bearing::from_deg(observer.bearing_to(&target).deg + noise_deg),
                stddev_rad: 0.5_f64.to_radians(),
            })
            .collect();

        let fix = bearing_least_squares_fix(&observations).expect("observable geometry");
        assert!(fix.position.distance_to(&target) < 0.5);
        assert!(fix.covariance.xx.is_finite());
        assert!(fix.covariance.yy.is_finite());
    }

    #[test]
    fn stationary_observer_is_unobservable() {
        let observations = vec![
            BearingLineObservation {
                observer: Position2D::zero(),
                bearing: Bearing::from_deg(30.0),
                stddev_rad: 1.0_f64.to_radians(),
            },
            BearingLineObservation {
                observer: Position2D::zero(),
                bearing: Bearing::from_deg(31.0),
                stddev_rad: 1.0_f64.to_radians(),
            },
        ];
        assert!(bearing_least_squares_fix(&observations).is_none());
    }

    #[test]
    fn bearing_rate_range_uses_correct_units() {
        let range = tma_bearing_only_tracking(
            10.0,
            Bearing::from_deg(0.0),
            Bearing::from_deg(90.0),
            0.01,
        )
        .expect("non-zero bearing rate");
        let expected = (10.0 * 1.852 / 3600.0) / 0.01_f64.to_radians();
        assert!((range - expected).abs() < 1.0e-9);
    }
}

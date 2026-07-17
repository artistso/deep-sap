//! Per-track bearing-only localization.
//! No truth-world positions or classifications are visible to this system.

use bevy::prelude::*;

use crate::api::ApiState;
use crate::components::*;
use crate::math::polynomial::ssp_polynomial_leroy;
use crate::math::trig::{
    bearing_least_squares_fix, Bearing, BearingLineObservation, Position2D,
};

#[derive(Resource, Debug, Default)]
pub struct LocatorState {
    pub tracks_with_fix: usize,
    pub last_update_s: f64,
}

pub fn locator_fusion_system(
    time: Res<Time>,
    mut locator_state: ResMut<LocatorState>,
    mut tracks: Query<&mut SapTrack>,
) {
    let now = time.elapsed_secs_f64();
    let mut tracks_with_fix = 0;

    for mut track in tracks.iter_mut() {
        let Some(latest_observation) = track.observations.last() else {
            continue;
        };
        if track
            .estimate
            .map(|estimate| estimate.updated_at_s >= latest_observation.observed_at_s)
            .unwrap_or(false)
        {
            if track.estimate.is_some() {
                tracks_with_fix += 1;
            }
            continue;
        }

        let recent_start = latest_observation.observed_at_s - 20.0;
        let observations: Vec<BearingLineObservation> = track
            .observations
            .iter()
            .filter(|sample| sample.observed_at_s >= recent_start)
            .map(|sample| BearingLineObservation {
                observer: Position2D::new(
                    sample.observer_position_km.x as f64,
                    sample.observer_position_km.y as f64,
                ),
                bearing: Bearing::from_deg(sample.bearing_deg as f64),
                stddev_rad: (sample.bearing_stddev_deg as f64).to_radians(),
            })
            .collect();

        let Some(fix) = bearing_least_squares_fix(&observations) else {
            continue;
        };
        if fix.rms_angular_residual_rad.to_degrees() > 8.0 {
            continue;
        }

        let position = Vec2::new(fix.position.x as f32, fix.position.y as f32);
        let previous_estimate = track.estimate;
        let velocity_km_s = previous_estimate
            .and_then(|previous| {
                let delta_time = latest_observation.observed_at_s - previous.updated_at_s;
                (delta_time > 0.1).then_some((position - previous.position_km) / delta_time as f32)
            })
            .map(|measured_velocity| {
                let previous_velocity = previous_estimate
                    .map(|estimate| estimate.velocity_km_s)
                    .unwrap_or(Vec2::ZERO);
                previous_velocity * 0.7 + measured_velocity * 0.3
            })
            .unwrap_or(Vec2::ZERO);

        let covariance = Covariance2 {
            xx: fix.covariance.xx,
            xy: fix.covariance.xy,
            yy: fix.covariance.yy,
        };
        let ellipse_95 = covariance_ellipse_95(covariance);
        track.estimate = Some(TrackEstimate {
            position_km: position,
            velocity_km_s,
            predicted_position_30s_km: position + velocity_km_s * 30.0,
            covariance,
            ellipse_95,
            updated_at_s: latest_observation.observed_at_s,
        });
        track.track_quality = track
            .track_quality
            .max(if fix.observations_used >= 12 { 4 } else { 3 });
        tracks_with_fix += 1;
    }

    locator_state.tracks_with_fix = tracks_with_fix;
    locator_state.last_update_s = now;
}

pub fn ssp_polynomial_system(
    mut ocean: ResMut<OceanProfile>,
    api_state: Res<ApiState>,
) {
    let environment = &api_state.environment;
    if let Some(surface_temperature) = environment.sst_c.value {
        ocean.surface_temp_c = surface_temperature;
    }
    if let Some(salinity) = environment.salinity_psu.value {
        ocean.salinity_psu = salinity;
    }
    if let Some(depth) = environment.bottom_depth_m.value {
        ocean.bottom_depth_m = depth.abs().max(50.0);
    }
    if let Some(wave_height) = environment.wave_height_m.value {
        ocean.wave_height_m = wave_height.max(0.0);
    }

    ocean.sound_speed_surface = ssp_polynomial_leroy(
        ocean.surface_temp_c,
        ocean.salinity_psu,
        0.0,
    );
}

fn covariance_ellipse_95(covariance: Covariance2) -> UncertaintyEllipse {
    let trace = covariance.xx + covariance.yy;
    let discriminant =
        ((covariance.xx - covariance.yy).powi(2) + 4.0 * covariance.xy.powi(2)).sqrt();
    let lambda_major = ((trace + discriminant) * 0.5).max(0.0);
    let lambda_minor = ((trace - discriminant) * 0.5).max(0.0);
    let angle = 0.5 * (2.0 * covariance.xy).atan2(covariance.xx - covariance.yy);
    let chi_square_95_scale = 5.991_f64.sqrt();

    UncertaintyEllipse {
        major_axis_km: (lambda_major.sqrt() * chi_square_95_scale) as f32,
        minor_axis_km: (lambda_minor.sqrt() * chi_square_95_scale) as f32,
        angle_rad: angle as f32,
        confidence: 0.95,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn covariance_becomes_oriented_ellipse() {
        let ellipse = covariance_ellipse_95(Covariance2 {
            xx: 4.0,
            xy: 0.0,
            yy: 1.0,
        });
        assert!(ellipse.major_axis_km > ellipse.minor_axis_km);
        assert!(ellipse.angle_rad.abs() < 1.0e-6);
        assert!((ellipse.confidence - 0.95).abs() < 1.0e-6);
    }
}

use bevy::prelude::*;

use crate::components::*;
use crate::math::trig::angular_distance_abs_deg;
use crate::systems::nav_bearing_deg;

const KNOT_TO_KM_PER_SECOND: f32 = 1.852 / 3600.0;

pub fn sap_fusion_system(
    mut commands: Commands,
    time: Res<Time>,
    mut sap_table: ResMut<SapTable>,
    mut observations: Query<(Entity, &mut BearingObservation)>,
    mut tracks: Query<(Entity, &mut SapTrack)>,
) {
    let now = time.elapsed_secs_f64();

    for (_observation_entity, mut observation) in observations.iter_mut() {
        if observation.consumed
            || observation.confidence < 0.12
            || observation.snr_db < -10.0
        {
            continue;
        }

        let best_track = {
            let mut best: Option<(Entity, f32)> = None;
            for (track_entity, track) in tracks.iter_mut() {
                if now - track.last_update_s > 20.0 {
                    continue;
                }

                let expected_bearing = track
                    .estimate
                    .map(|estimate| {
                        nav_bearing_deg(
                            estimate.position_km - observation.observer_position_km,
                        )
                    })
                    .unwrap_or(track.last_bearing);
                let angular_error = angular_distance_abs_deg(
                    expected_bearing as f64,
                    observation.bearing_deg as f64,
                ) as f32;
                let frequency_error =
                    (track.last_frequency_hz - observation.freq_hz).abs();
                let angular_gate = if track.estimate.is_some() { 6.0 } else { 9.0 };
                let frequency_gate = 18.0;
                let score = angular_error / angular_gate + frequency_error / frequency_gate;

                if angular_error <= angular_gate
                    && frequency_error <= frequency_gate
                    && best.map(|(_, best_score)| score < best_score).unwrap_or(true)
                {
                    best = Some((track_entity, score));
                }
            }
            best.map(|(entity, _)| entity)
        };

        if let Some(track_entity) = best_track {
            if let Ok((_, mut track)) = tracks.get_mut(track_entity) {
                update_track_from_observation(&mut track, &observation);
                observation.consumed = true;
            }
        } else if sap_table.tracks.len() < 24 {
            sap_table.total_tracks_created += 1;
            let track_number = sap_table.total_tracks_created;
            let id = format!("TRACK-{track_number:03}");
            let submarine_probability = observation_evidence(&observation, None);
            let entity = commands
                .spawn((
                    SapTrack {
                        number: track_number,
                        id: id.clone(),
                        track_quality: 1,
                        last_bearing: observation.bearing_deg,
                        last_frequency_hz: observation.freq_hz,
                        last_update_s: observation.observed_at_s,
                        submarine_probability,
                        observations: vec![bearing_sample(&observation)],
                        estimate: None,
                    },
                    Classification {
                        state: ClassState::Unknown,
                        certainty: observation.confidence * 0.5,
                    },
                    Name::new(id),
                ))
                .id();
            sap_table.tracks.push(entity);
            observation.consumed = true;
        }
    }

    let mut stale_tracks = Vec::new();
    let mut probable_submarines = 0;
    for (entity, track) in tracks.iter_mut() {
        if now - track.last_update_s > 45.0 {
            stale_tracks.push(entity);
        } else if track.submarine_probability > 0.7 && track.track_quality >= 3 {
            probable_submarines += 1;
        }
    }
    for entity in stale_tracks {
        commands.entity(entity).despawn();
        sap_table.tracks.retain(|candidate| *candidate != entity);
    }
    sap_table.probable_submarines = probable_submarines;
}

pub fn classification_update_system(
    mut tracks: Query<(&SapTrack, &mut Classification)>,
) {
    for (track, mut classification) in tracks.iter_mut() {
        let (state, certainty) = if track.submarine_probability >= 0.72 {
            (
                ClassState::SyntheticSubmarine,
                track.submarine_probability,
            )
        } else if track.submarine_probability <= 0.28 {
            (ClassState::Biologic, 1.0 - track.submarine_probability)
        } else {
            (ClassState::Unknown, 1.0 - (track.submarine_probability - 0.5).abs() * 2.0)
        };
        classification.state = state;
        classification.certainty = certainty.clamp(0.0, 1.0);
    }
}

fn update_track_from_observation(track: &mut SapTrack, observation: &BearingObservation) {
    let previous_frequency = track.last_frequency_hz;
    let evidence = observation_evidence(observation, Some(previous_frequency));
    track.submarine_probability =
        (track.submarine_probability * 0.82 + evidence * 0.18).clamp(0.01, 0.99);
    track.last_bearing = observation.bearing_deg;
    track.last_frequency_hz = observation.freq_hz;
    track.last_update_s = observation.observed_at_s;
    track.observations.push(bearing_sample(observation));
    if track.observations.len() > 40 {
        track.observations.remove(0);
    }
    track.track_quality = match track.observations.len() {
        0..=2 => 1,
        3..=5 => 2,
        6..=11 => 3,
        12..=19 => 4,
        _ => 5,
    };
}

fn bearing_sample(observation: &BearingObservation) -> BearingSample {
    BearingSample {
        observer_position_km: observation.observer_position_km,
        bearing_deg: observation.bearing_deg,
        bearing_stddev_deg: observation.bearing_stddev_deg,
        observed_at_s: observation.observed_at_s,
    }
}

fn observation_evidence(
    observation: &BearingObservation,
    previous_frequency_hz: Option<f32>,
) -> f32 {
    let tonal_band_score = if (45.0..=180.0).contains(&observation.freq_hz) {
        0.85
    } else if observation.freq_hz < 35.0 {
        0.12
    } else {
        0.45
    };
    let stability = previous_frequency_hz
        .map(|previous| 1.0 - ((previous - observation.freq_hz).abs() / 20.0).clamp(0.0, 1.0))
        .unwrap_or(0.5);
    let quality = observation.confidence.clamp(0.0, 1.0);
    (tonal_band_score * 0.55 + stability * 0.25 + quality * 0.20).clamp(0.0, 1.0)
}

pub fn tma_system(
    time: Res<Time>,
    ocean: Res<OceanProfile>,
    mut targets: Query<
        (&TargetId, &mut Position, &mut Velocity, &mut Depth, &Submarine),
        With<TruthTarget>,
    >,
) {
    let delta_seconds = time.delta_secs();
    let elapsed_seconds = time.elapsed_secs();

    for (target_id, mut position, mut velocity, mut depth, target) in targets.iter_mut() {
        match target.class {
            SubClass::SsnAkula => {
                if position.0.length() < 12.0 {
                    velocity.0 += position.0.normalize_or_zero() * 0.02 * delta_seconds;
                }
                if depth.0 < ocean.thermocline_depth_m + 20.0 {
                    depth.0 += 0.3 * delta_seconds;
                }
                clamp_speed_knots(&mut velocity.0, 12.0);
            }
            SubClass::SsbnBorei => {
                let target_depth = (ocean.bottom_depth_m * 0.15).clamp(120.0, 500.0);
                depth.0 += (target_depth - depth.0) * (0.002 * delta_seconds).clamp(0.0, 1.0);
                clamp_speed_knots(&mut velocity.0, 6.0);
            }
            SubClass::SskKilo => {
                let phase = (elapsed_seconds * 0.005 + target_id.0 as f32).sin();
                if phase > 0.999 && depth.0 > 50.0 {
                    depth.0 -= 0.15 * delta_seconds;
                }
                clamp_speed_knots(&mut velocity.0, 9.0);
            }
            SubClass::Whale => {
                let turn_rate =
                    (elapsed_seconds * 0.17 + target_id.0 as f32 * 0.31).sin() * 0.08;
                velocity.0 = Mat2::from_angle(turn_rate * delta_seconds) * velocity.0;
                clamp_speed_knots(&mut velocity.0, 3.0);
            }
            SubClass::Uuv => {
                clamp_speed_knots(&mut velocity.0, 10.0);
            }
        }

        position.0 += velocity.0 * KNOT_TO_KM_PER_SECOND * delta_seconds;
        depth.0 = depth.0.clamp(10.0, (ocean.bottom_depth_m - 50.0).max(10.0));
    }
}

fn clamp_speed_knots(velocity: &mut Vec2, maximum: f32) {
    let speed = velocity.length();
    if speed > maximum {
        *velocity = velocity.normalize_or_zero() * maximum;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_minute_at_ten_knots_is_not_exaggerated() {
        let distance_km = 10.0 * KNOT_TO_KM_PER_SECOND * 60.0;
        assert!((distance_km - 0.308_666_68).abs() < 1.0e-6);
    }
}

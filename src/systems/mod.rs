pub mod locator;

use bevy::prelude::*;
use rand::prelude::*;
use rand::rngs::SmallRng;

use crate::components::*;

pub use locator::{locator_fusion_system, ssp_polynomial_system, LocatorState};

const KNOT_TO_KM_PER_SECOND: f32 = 1.852 / 3600.0;
const SENSOR_SWEEP_SECONDS: f32 = 0.5;

pub fn spawn_mission(
    mut commands: Commands,
    ocean: Res<OceanProfile>,
    mission: Res<MissionConfig>,
) {
    commands.spawn((
        Ownship,
        Position(Vec2::ZERO),
        Velocity(nav_vector(25.0) * 8.0),
        Depth(15.0),
        TowedArray,
        Name::new("OWNSHIP"),
    ));

    let mut rng = SmallRng::seed_from_u64(mission.seed);

    for index in 0..mission.hidden_subs {
        let bearing_deg = rng.gen_range(0.0..360.0);
        let range_km = rng.gen_range(8.0..28.0);
        let position = nav_vector(bearing_deg) * range_km;
        let class = if index == 0 {
            SubClass::SsbnBorei
        } else if index % 3 == 0 {
            SubClass::SskKilo
        } else {
            SubClass::SsnAkula
        };
        let depth = if class == SubClass::SsbnBorei {
            rng.gen_range(250.0..400.0)
        } else {
            ocean.thermocline_depth_m + rng.gen_range(-30.0..80.0)
        };
        let quieting_factor = if class == SubClass::SsbnBorei {
            0.9
        } else {
            0.7
        };

        commands.spawn((
            TruthTarget,
            TargetId(index as u32 + 1),
            Submarine {
                class,
                faction: Faction::Hostile,
                stealth: 70.0 + mission.difficulty * 20.0,
            },
            Position(position),
            Velocity(nav_vector(rng.gen_range(0.0..360.0)) * rng.gen_range(3.0..8.0)),
            Depth(depth),
            AcousticSignature {
                broadband_db: if class == SubClass::SsbnBorei {
                    164.0
                } else {
                    170.0
                },
                blade_count: 7,
                shaft_hz: if class == SubClass::SsbnBorei {
                    rng.gen_range(8.0..12.0)
                } else {
                    rng.gen_range(12.0..20.0)
                },
                quieting_factor,
                is_cavitating: false,
            },
            Name::new(format!("SYNTHETIC_TARGET_{index}")),
        ));
    }

    for index in 0..mission.biologics {
        let position = Vec2::new(rng.gen_range(-20.0..20.0), rng.gen_range(-20.0..20.0));
        commands.spawn((
            TruthTarget,
            TargetId(10_000 + index as u32),
            Submarine {
                class: SubClass::Whale,
                faction: Faction::Biologic,
                stealth: 10.0,
            },
            Position(position),
            Velocity(Vec2::new(
                rng.gen_range(-2.0..2.0),
                rng.gen_range(-2.0..2.0),
            )),
            Depth(rng.gen_range(10.0..100.0)),
            AcousticSignature {
                broadband_db: rng.gen_range(150.0..160.0),
                blade_count: 0,
                shaft_hz: rng.gen_range(5.0..30.0),
                quieting_factor: 0.0,
                is_cavitating: false,
            },
            Name::new(format!("SYNTHETIC_BIOLOGIC_{index}")),
        ));
    }
}

pub fn ownship_motion_system(
    time: Res<Time>,
    mut ownship: Query<(&mut Position, &Velocity), With<Ownship>>,
) {
    let Ok((mut position, velocity)) = ownship.get_single_mut() else {
        return;
    };
    position.0 += velocity.0 * KNOT_TO_KM_PER_SECOND * time.delta_secs();
}

pub fn camera_follow_ownship_system(
    ownship_query: Query<&Position, With<Ownship>>,
    mut camera_query: Query<&mut Transform, With<Camera2d>>,
) {
    let Ok(ownship_position) = ownship_query.get_single() else {
        return;
    };
    let Ok(mut camera_transform) = camera_query.get_single_mut() else {
        return;
    };
    camera_transform.translation.x = ownship_position.0.x;
    camera_transform.translation.y = ownship_position.0.y;
}

pub fn acoustic_propagation_system(
    mut commands: Commands,
    time: Res<Time>,
    ocean: Res<OceanProfile>,
    ownship_query: Query<&Position, With<Ownship>>,
    target_query: Query<
        (&TargetId, &Position, &Depth, &AcousticSignature, &Submarine),
        With<TruthTarget>,
    >,
    mut sweep_accumulator: Local<f32>,
) {
    *sweep_accumulator += time.delta_secs();
    if *sweep_accumulator < SENSOR_SWEEP_SECONDS {
        return;
    }
    *sweep_accumulator = (*sweep_accumulator - SENSOR_SWEEP_SECONDS).max(0.0);

    let Ok(ownship_position) = ownship_query.get_single() else {
        return;
    };
    let sweep_index = (time.elapsed_secs_f64() / SENSOR_SWEEP_SECONDS as f64).floor() as u64;

    for (target_id, position, depth, signature, target) in target_query.iter() {
        let offset = position.0 - ownship_position.0;
        let range_km = offset.length().max(0.05);
        let true_bearing_deg = nav_bearing_deg(offset);
        let tonal_frequency_hz = if signature.blade_count == 0 {
            signature.shaft_hz
        } else {
            signature.shaft_hz * signature.blade_count as f32
        };
        let transmission_loss =
            ocean.transmission_loss(range_km, depth.0, tonal_frequency_hz.max(1.0));
        let ambient_noise_db = 50.0 + ocean.wave_height_m.clamp(0.0, 8.0) * 1.8;
        let array_gain_db = 35.0 + if range_km > 10.0 { 3.0 } else { 0.0 };
        let effective_source_level = signature.broadband_db
            - target.stealth * signature.quieting_factor * 0.15
            + if signature.is_cavitating { 12.0 } else { 0.0 };
        let snr_db = effective_source_level - transmission_loss - ambient_noise_db + array_gain_db;
        let confidence = ((snr_db + 12.0) / 24.0).clamp(0.0, 1.0);

        let detection_draw = unit_noise(target_id.0 as u64, sweep_index, 71);
        if confidence < 0.08 || detection_draw > (0.15 + confidence * 0.85) {
            continue;
        }

        let bearing_stddev_deg = 4.5 - confidence * 4.0;
        let bearing_noise = gaussianish_noise(target_id.0 as u64, sweep_index, 13)
            * bearing_stddev_deg;
        let frequency_noise = gaussianish_noise(target_id.0 as u64, sweep_index, 29)
            * (0.5 + (1.0 - confidence) * 3.0);

        commands.spawn((
            BearingObservation {
                sensor: SensorKind::TowedArray,
                observer_position_km: ownship_position.0,
                bearing_deg: (true_bearing_deg + bearing_noise).rem_euclid(360.0),
                bearing_stddev_deg,
                snr_db,
                freq_hz: (tonal_frequency_hz + frequency_noise).max(0.0),
                confidence,
                observed_at_s: time.elapsed_secs_f64(),
                age_s: 0.0,
                consumed: false,
            },
            Name::new("BEARING_OBSERVATION"),
        ));
    }
}

pub fn input_s_pen_system(
    mut input_state: ResMut<InputState>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    touch_input: Res<Touches>,
    windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    ownship_query: Query<&Position, With<Ownship>>,
) {
    let Ok(window) = windows.get_single() else {
        return;
    };
    let Ok((camera, camera_transform)) = camera_query.get_single() else {
        return;
    };
    let Ok(ownship_position) = ownship_query.get_single() else {
        return;
    };

    let cursor_position = touch_input
        .iter()
        .next()
        .map(|touch| touch.position())
        .or_else(|| window.cursor_position());

    if let Some(position) = cursor_position {
        if let Ok(world_position) = camera.viewport_to_world_2d(camera_transform, position) {
            input_state.last_pen_pos = Some(world_position);
            input_state.active_bearing =
                nav_bearing_deg(world_position - ownship_position.0);
        }
    }

    input_state.is_pen_down =
        mouse_button.pressed(MouseButton::Left) || touch_input.any_just_pressed();
    input_state.pressure = if input_state.is_pen_down { 0.5 } else { 0.0 };
    update_wasm_stylus_state(&mut input_state);
}

pub fn render_sonar_background(
    mut gizmos: Gizmos,
    ocean: Res<OceanProfile>,
    input_state: Res<InputState>,
    observations: Query<&BearingObservation>,
    tracks: Query<&SapTrack>,
    ownship_query: Query<&Position, With<Ownship>>,
) {
    let Ok(ownship_position) = ownship_query.get_single() else {
        return;
    };
    let center = ownship_position.0;

    for degrees in (0..360).step_by(30) {
        let direction = nav_vector(degrees as f32);
        let inner_radius = 30.0;
        let outer_radius = if degrees % 90 == 0 { 35.0 } else { 32.0 };
        gizmos.line_2d(
            center + direction * inner_radius,
            center + direction * outer_radius,
            Color::srgba(0.2, 0.5, 0.8, 0.3),
        );
    }

    for radius in [5.0, 10.0, 15.0, 20.0, 30.0] {
        gizmos
            .circle_2d(
                center,
                radius,
                Color::srgba(0.1, 0.4, 0.7, 0.15),
            )
            .resolution(128);
    }

    gizmos
        .circle_2d(
            center,
            ocean.thermocline_depth_m * 0.05 + 5.0,
            Color::srgba(0.3, 0.7, 1.0, 0.08),
        )
        .resolution(64);

    if input_state.last_pen_pos.is_some() {
        let direction = nav_vector(input_state.active_bearing);
        gizmos.line_2d(
            center,
            center + direction * 40.0,
            Color::srgba(0.9, 1.0, 0.5, 0.9),
        );
        gizmos.circle_2d(
            center + direction * 40.0,
            0.4 + input_state.pressure * 0.4,
            Color::srgb(1.0, 1.0, 0.4),
        );
    }

    for observation in observations.iter() {
        if observation.snr_db < -8.0 || observation.age_s > 8.0 {
            continue;
        }
        let direction = nav_vector(observation.bearing_deg);
        let alpha = observation.confidence.clamp(0.08, 0.65) * (1.0 - observation.age_s / 8.0);
        gizmos.line_2d(
            observation.observer_position_km,
            observation.observer_position_km + direction * 35.0,
            Color::srgba(0.3, 0.75, 1.0, alpha * 0.25),
        );
    }

    for track in tracks.iter() {
        let Some(estimate) = track.estimate else {
            continue;
        };
        let color = if track.submarine_probability > 0.7 {
            Color::srgba(1.0, 0.5, 0.2, 0.9)
        } else {
            Color::srgba(0.3, 1.0, 0.75, 0.8)
        };
        gizmos.circle_2d(estimate.position_km, 0.45, color);
        gizmos.line_2d(
            estimate.position_km,
            estimate.predicted_position_30s_km,
            color.with_alpha(0.6),
        );
        gizmos.circle_2d(
            estimate.predicted_position_30s_km,
            0.25,
            color.with_alpha(0.55),
        );
        draw_uncertainty_ellipse(
            &mut gizmos,
            estimate.position_km,
            estimate.ellipse_95,
            color.with_alpha(0.25),
        );
    }

    gizmos.circle_2d(center, 0.8, Color::srgb(0.2, 1.0, 0.6));
    gizmos.circle_2d(
        center,
        1.2,
        Color::srgba(0.2, 1.0, 0.6, 0.3),
    );
}

pub fn cleanup_old_observations(
    mut commands: Commands,
    time: Res<Time>,
    mut observations: Query<(Entity, &mut BearingObservation)>,
) {
    for (entity, mut observation) in observations.iter_mut() {
        observation.age_s += time.delta_secs();
        if observation.age_s > 20.0 {
            commands.entity(entity).despawn();
        }
    }
}

pub fn nav_vector(bearing_deg: f32) -> Vec2 {
    let radians = bearing_deg.to_radians();
    Vec2::new(radians.sin(), radians.cos())
}

pub fn nav_bearing_deg(vector: Vec2) -> f32 {
    vector.x.atan2(vector.y).to_degrees().rem_euclid(360.0)
}

fn draw_uncertainty_ellipse(
    gizmos: &mut Gizmos,
    center: Vec2,
    ellipse: UncertaintyEllipse,
    color: Color,
) {
    let major = ellipse.major_axis_km.clamp(0.05, 100.0);
    let minor = ellipse.minor_axis_km.clamp(0.05, 100.0);
    let rotation = Mat2::from_angle(ellipse.angle_rad);
    let segments = 64;
    let mut previous = center + rotation * Vec2::new(major, 0.0);
    for index in 1..=segments {
        let angle = std::f32::consts::TAU * index as f32 / segments as f32;
        let current = center
            + rotation * Vec2::new(major * angle.cos(), minor * angle.sin());
        gizmos.line_2d(previous, current, color);
        previous = current;
    }
}

fn unit_noise(target: u64, sweep: u64, channel: u64) -> f32 {
    let value = target
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add(sweep.wrapping_mul(0xBF58_476D_1CE4_E5B9))
        .wrapping_add(channel.wrapping_mul(0x94D0_49BB_1331_11EB));
    let mixed = splitmix64(value);
    (mixed as f64 / u64::MAX as f64) as f32
}

fn gaussianish_noise(target: u64, sweep: u64, channel: u64) -> f32 {
    let sum = (0..6)
        .map(|offset| unit_noise(target, sweep, channel + offset))
        .sum::<f32>();
    (sum - 3.0) * 0.82
}

fn splitmix64(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9E37_79B9_7F4A_7C15);
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

#[cfg(target_arch = "wasm32")]
fn update_wasm_stylus_state(input_state: &mut InputState) {
    use js_sys::Reflect;
    use wasm_bindgen::JsValue;

    let Some(window) = web_sys::window() else {
        return;
    };
    let window_value: &JsValue = window.as_ref();
    let Ok(stylus) = Reflect::get(window_value, &JsValue::from_str("__deepSapStylus")) else {
        return;
    };
    if stylus.is_undefined() || stylus.is_null() {
        return;
    }

    let number = |name: &str| -> Option<f64> {
        Reflect::get(&stylus, &JsValue::from_str(name))
            .ok()
            .and_then(|value| value.as_f64())
    };
    let boolean = |name: &str| -> Option<bool> {
        Reflect::get(&stylus, &JsValue::from_str(name))
            .ok()
            .and_then(|value| value.as_bool())
    };

    input_state.pen_active = boolean("active").unwrap_or(false);
    input_state.pressure = number("pressure").unwrap_or(0.0) as f32;
    input_state.tilt_x_deg = number("tiltX").unwrap_or(0.0) as f32;
    input_state.tilt_y_deg = number("tiltY").unwrap_or(0.0) as f32;
    input_state.twist_deg = number("twist").unwrap_or(0.0) as f32;
    input_state.barrel_button = boolean("barrelButton").unwrap_or(false);
    input_state.hovering = boolean("hovering").unwrap_or(false);
}

#[cfg(not(target_arch = "wasm32"))]
fn update_wasm_stylus_state(_input_state: &mut InputState) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn knot_conversion_matches_one_hour_distance() {
        let distance = 10.0 * KNOT_TO_KM_PER_SECOND * 3600.0;
        assert!((distance - 18.52).abs() < 1.0e-5);
    }

    #[test]
    fn navigation_bearing_round_trip() {
        for bearing in [0.0, 45.0, 90.0, 180.0, 270.0, 359.0] {
            let recovered = nav_bearing_deg(nav_vector(bearing));
            assert!((recovered - bearing).abs() < 1.0e-4);
        }
    }
}

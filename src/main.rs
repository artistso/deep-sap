#![allow(dead_code)]

mod api;
mod components;
mod math;
mod sap;
mod systems;

use api::{ApiPlugin, ApiState, DataQuality};
use bevy::prelude::*;
use bevy::render::camera::ScalingMode;
use components::*;
use sap::*;
use systems::*;

fn main() {
    let mut app = App::new();

    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "DEEP SAP — Synthetic Ocean Sensor Fusion".into(),
                    canvas: Some("#bevy".into()),
                    fit_canvas_to_parent: true,
                    prevent_default_event_handling: false,
                    resolution: bevy::window::WindowResolution::new(1752.0, 2800.0)
                        .with_scale_factor_override(1.0),
                    ..default()
                }),
                ..default()
            })
            .set(ImagePlugin::default_nearest()),
    )
    .add_plugins(ApiPlugin)
    .insert_resource(OceanProfile::default())
    .insert_resource(SapTable::default())
    .insert_resource(InputState::default())
    .insert_resource(MissionConfig {
        hidden_subs: 4,
        biologics: 8,
        difficulty: 0.8,
        seed: 42,
    })
    .insert_resource(ClearColor(Color::srgb(0.001, 0.02, 0.05)))
    .insert_resource(LocatorState::default())
    .add_systems(Startup, (setup_camera, spawn_mission, setup_ui))
    .add_systems(
        Update,
        (
            ssp_polynomial_system,
            ownship_motion_system,
            camera_follow_ownship_system,
            tma_system,
            acoustic_propagation_system,
            sap_fusion_system,
            locator_fusion_system,
            classification_update_system,
            input_s_pen_system,
            render_sonar_background,
            ui_update_system,
            waterfall_update,
            cleanup_old_observations,
        )
            .chain(),
    );

    #[cfg(target_arch = "wasm32")]
    app.insert_resource(bevy::winit::WinitSettings {
        focused_mode: bevy::winit::UpdateMode::Continuous,
        unfocused_mode: bevy::winit::UpdateMode::Reactive {
            wait: std::time::Duration::from_millis(50),
            react_to_device_events: true,
            react_to_user_events: true,
            react_to_window_events: true,
        },
    });

    app.run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Projection::from(OrthographicProjection {
            scaling_mode: ScalingMode::FixedVertical {
                viewport_height: 80.0,
            },
            near: -1000.0,
            far: 1000.0,
            ..OrthographicProjection::default_2d()
        }),
        Msaa::Off,
        Transform::from_xyz(0.0, 0.0, 999.0),
    ));
}

#[derive(Component)]
struct UiRoot;

#[derive(Component)]
struct SapTextMarker;

fn setup_ui(mut commands: Commands) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            },
            UiRoot,
        ))
        .with_children(|parent| {
            parent
                .spawn(Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(72.0),
                    padding: UiRect::all(Val::Px(12.0)),
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::SpaceBetween,
                    ..default()
                })
                .with_children(|top| {
                    top.spawn((
                        Text::new("DEEP SAP // SYNTHETIC SENSOR-FUSION LAB"),
                        TextFont {
                            font_size: 14.0,
                            ..default()
                        },
                        TextColor(Color::srgba(0.6, 0.9, 1.0, 0.9)),
                    ));
                    top.spawn((
                        Text::new("UNCLASSIFIED SIMULATION // S PEN POINTER INPUT"),
                        TextFont {
                            font_size: 10.0,
                            ..default()
                        },
                        TextColor(Color::srgba(0.5, 0.7, 0.9, 0.6)),
                    ));
                });

            parent
                .spawn(Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(150.0),
                    flex_direction: FlexDirection::Column,
                    ..default()
                })
                .with_children(|bottom| {
                    bottom
                        .spawn((
                            Node {
                                width: Val::Percent(100.0),
                                height: Val::Px(94.0),
                                border: UiRect::all(Val::Px(1.0)),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.02, 0.08, 0.15, 0.9)),
                        ))
                        .with_children(|waterfall| {
                            waterfall.spawn((
                                Text::new(
                                    "PASSIVE SYNTHETIC ARRAY // NOISY BEARINGS // PUBLIC DATA CONTROLS ENVIRONMENT ONLY",
                                ),
                                TextFont {
                                    font_size: 11.0,
                                    ..default()
                                },
                                TextColor(Color::srgba(0.4, 0.7, 1.0, 0.7)),
                            ));
                        });

                    bottom
                        .spawn((
                            Node {
                                width: Val::Percent(100.0),
                                height: Val::Px(56.0),
                                padding: UiRect::all(Val::Px(6.0)),
                                flex_direction: FlexDirection::Row,
                                column_gap: Val::Px(12.0),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.0, 0.1, 0.2, 0.7)),
                        ))
                        .with_children(|status| {
                            status.spawn((
                                Text::new("TRACKER INITIALIZING"),
                                TextFont {
                                    font_size: 12.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.9, 0.9, 0.6)),
                                SapTextMarker,
                            ));
                        });
                });
        });
}

#[allow(clippy::too_many_arguments)]
fn ui_update_system(
    time: Res<Time>,
    input_state: Res<InputState>,
    ocean: Res<OceanProfile>,
    observations: Query<&BearingObservation>,
    sap_table: Res<SapTable>,
    locator_state: Res<LocatorState>,
    api_state: Res<ApiState>,
    tracks: Query<&SapTrack>,
    mut status_text: Query<&mut Text, With<SapTextMarker>>,
) {
    let active_observations = observations
        .iter()
        .filter(|observation| observation.age_s < 5.0 && observation.snr_db > -8.0)
        .count();
    let data_mode = match api_state.environment.sst_c.quality {
        DataQuality::Observed => "PUBLIC-LIVE",
        DataQuality::Synthetic => "OFFLINE-SYNTH",
        DataQuality::Stale => "PUBLIC-STALE",
        DataQuality::Unknown => "NO-DATA",
    };

    let mut text =
        format!(
        "T={:.0}s | SST {:.1}C | SAL {:.1} | OBS {} | FIXES {} | BRG {:.0}° | PEN {} | DATA {} | ",
        time.elapsed_secs(),
        ocean.surface_temp_c,
        ocean.salinity_psu,
        active_observations,
        locator_state.tracks_with_fix,
        input_state.active_bearing,
        if input_state.pen_active {
            if input_state.hovering { "HOVER" } else { "ACTIVE" }
        } else {
            "POINTER"
        },
        data_mode,
    );

    if sap_table.tracks.is_empty() {
        text.push_str("NO TRACKS — WAITING FOR OBSERVABLE GEOMETRY");
    } else {
        for entity in sap_table.tracks.iter().take(4) {
            if let Ok(track) = tracks.get(*entity) {
                let fix = if track.estimate.is_some() {
                    "FIX"
                } else {
                    "BRG"
                };
                text.push_str(&format!(
                    "[{} {:03.0}° Q{} Psub {:02.0}% {}] ",
                    track.id,
                    track.last_bearing,
                    track.track_quality,
                    track.submarine_probability * 100.0,
                    fix,
                ));
            }
        }
        text.push_str(&format!(
            "PROBABLE-SYNTH-SUBS {}",
            sap_table.probable_submarines
        ));
    }

    for mut label in status_text.iter_mut() {
        **label = text.clone();
    }
}

fn waterfall_update(mut gizmos: Gizmos, time: Res<Time>, observations: Query<&BearingObservation>) {
    let y_base = -38.0;
    let now = time.elapsed_secs();
    for observation in observations.iter() {
        if observation.snr_db < -10.0 || observation.age_s > 12.0 {
            continue;
        }
        let x = observation.bearing_deg / 360.0 * 80.0 - 40.0;
        let y = y_base + 8.0 - observation.age_s * 0.65;
        let intensity = observation.confidence.clamp(0.0, 1.0);
        gizmos.line_2d(
            Vec2::new(x, y),
            Vec2::new(x, y - 0.12 - intensity * 0.5),
            Color::srgba(0.2 + intensity * 0.8, 0.8, 1.0, intensity * 0.85),
        );
    }
    if (now as i32) % 2 == 0 {
        gizmos.line_2d(
            Vec2::new(-41.0, y_base),
            Vec2::new(41.0, y_base),
            Color::srgba(0.3, 0.5, 0.8, 0.4),
        );
    }
}

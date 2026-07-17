use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Scenario truth. Systems outside measurement generation and scoring must not
/// query this value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Faction {
    Friendly,
    Neutral,
    Hostile,
    Biologic,
}

/// Synthetic target archetypes. These are deliberately generic game labels,
/// not real acoustic libraries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubClass {
    SsnAkula,
    SsbnBorei,
    SskKilo,
    Uuv,
    Whale,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TargetId(pub u32);

/// Marker for entities that belong to the hidden synthetic truth world.
#[derive(Component, Debug, Clone, Copy)]
pub struct TruthTarget;

#[derive(Component, Debug, Clone, Copy)]
pub struct Position(pub Vec2); // km east/north from scenario origin

#[derive(Component, Debug, Clone, Copy)]
pub struct Velocity(pub Vec2); // knots east/north

#[derive(Component, Debug, Clone, Copy)]
pub struct Depth(pub f32); // metres

#[derive(Component, Debug, Clone)]
pub struct AcousticSignature {
    pub broadband_db: f32,
    pub blade_count: u8,
    pub shaft_hz: f32,
    pub quieting_factor: f32,
    pub is_cavitating: bool,
}

#[derive(Component, Debug, Clone)]
pub struct Submarine {
    pub class: SubClass,
    pub faction: Faction,
    pub stealth: f32,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct Ownship;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SensorKind {
    TowedArray,
    SyntheticBuoy,
}

/// A noisy sensor-domain observation. It intentionally contains no target ID,
/// truth position, faction, or class.
#[derive(Component, Debug, Clone)]
pub struct BearingObservation {
    pub sensor: SensorKind,
    pub observer_position_km: Vec2,
    pub bearing_deg: f32,
    pub bearing_stddev_deg: f32,
    pub snr_db: f32,
    pub freq_hz: f32,
    pub confidence: f32,
    pub observed_at_s: f64,
    pub age_s: f32,
    pub consumed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClassState {
    Unknown,
    Biologic,
    NeutralVessel,
    SyntheticSubmarine,
}

#[derive(Component, Debug, Clone)]
pub struct Classification {
    pub state: ClassState,
    pub certainty: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct BearingSample {
    pub observer_position_km: Vec2,
    pub bearing_deg: f32,
    pub bearing_stddev_deg: f32,
    pub observed_at_s: f64,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Covariance2 {
    pub xx: f64,
    pub xy: f64,
    pub yy: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct UncertaintyEllipse {
    pub major_axis_km: f32,
    pub minor_axis_km: f32,
    pub angle_rad: f32,
    pub confidence: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct TrackEstimate {
    pub position_km: Vec2,
    pub velocity_km_s: Vec2,
    pub predicted_position_30s_km: Vec2,
    pub covariance: Covariance2,
    pub ellipse_95: UncertaintyEllipse,
    pub updated_at_s: f64,
}

#[derive(Component, Debug, Clone)]
pub struct SapTrack {
    pub number: u32,
    pub id: String,
    pub track_quality: u8,
    pub last_bearing: f32,
    pub last_frequency_hz: f32,
    pub last_update_s: f64,
    pub submarine_probability: f32,
    pub observations: Vec<BearingSample>,
    pub estimate: Option<TrackEstimate>,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct TowedArray;

#[derive(Resource, Debug, Clone)]
pub struct OceanProfile {
    pub surface_temp_c: f32,
    pub salinity_psu: f32,
    pub thermocline_depth_m: f32,
    pub thermocline_strength_c: f32,
    pub bottom_depth_m: f32,
    pub sound_speed_surface: f32,
    pub wave_height_m: f32,
}

impl Default for OceanProfile {
    fn default() -> Self {
        Self {
            surface_temp_c: 14.0,
            salinity_psu: 33.5,
            thermocline_depth_m: 120.0,
            thermocline_strength_c: 8.0,
            bottom_depth_m: 3500.0,
            sound_speed_surface: 1500.0,
            wave_height_m: 1.5,
        }
    }
}

impl OceanProfile {
    pub fn sound_speed_at_depth(&self, depth_m: f32) -> f32 {
        if depth_m < self.thermocline_depth_m {
            self.sound_speed_surface - 0.5 * depth_m / self.thermocline_depth_m.max(1.0)
        } else {
            self.sound_speed_surface
                - 0.5
                - 0.016 * (depth_m - self.thermocline_depth_m).min(200.0)
                + 0.014 * (depth_m - self.thermocline_depth_m - 200.0).max(0.0)
        }
    }

    pub fn transmission_loss(&self, range_km: f32, depth_m: f32, freq_hz: f32) -> f32 {
        let range_m = range_km.max(0.001) * 1000.0;
        let spherical = 20.0 * range_m.log10();
        let absorption = 0.001 * (freq_hz / 1000.0).max(0.0) * range_km;
        let thermocline_loss =
            if depth_m > self.thermocline_depth_m + 20.0 && range_km > 5.0 {
                10.0
            } else {
                0.0
            };
        spherical + absorption + thermocline_loss
    }
}

#[derive(Resource, Default)]
pub struct SapTable {
    pub tracks: Vec<Entity>,
    pub total_tracks_created: u32,
    pub probable_submarines: u32,
}

#[derive(Resource)]
pub struct GameClock {
    pub elapsed: f32,
    pub mission_time: f32,
}

#[derive(Resource, Default)]
pub struct InputState {
    pub last_pen_pos: Option<Vec2>,
    pub is_pen_down: bool,
    pub active_bearing: f32,
    pub pressure: f32,
    pub tilt_x_deg: f32,
    pub tilt_y_deg: f32,
    pub twist_deg: f32,
    pub barrel_button: bool,
    pub hovering: bool,
    pub pen_active: bool,
}

#[derive(Resource)]
pub struct MissionConfig {
    pub hidden_subs: usize,
    pub biologics: usize,
    pub difficulty: f32,
    pub seed: u64,
}

impl Default for MissionConfig {
    fn default() -> Self {
        Self {
            hidden_subs: 3,
            biologics: 7,
            difficulty: 0.75,
            seed: 42,
        }
    }
}

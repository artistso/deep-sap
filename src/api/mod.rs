//! Public environmental data integration.
//! External data affects environmental context only; all game targets remain
//! synthetic and are never inferred from public feeds.

// Only providers used by the shipping application are compiled. Historical
// credential-bearing and defense-named experiments were removed from the release package.
pub mod gebco;
pub mod noaa;

use bevy::prelude::*;
use bevy::tasks::{IoTaskPool, Task};
use futures_lite::future;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RealOceanSample {
    pub lat: f64,
    pub lon: f64,
    pub sst_c: Option<f32>,
    pub salinity_psu: Option<f32>,
    pub bottom_depth_m: Option<f32>,
    pub wave_height_m: Option<f32>,
    pub water_temp_c: Option<f32>,
    pub timestamp: String,
    pub source: String,
    pub doi: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DataQuality {
    #[default]
    Unknown,
    Synthetic,
    Observed,
    Stale,
}

#[derive(Debug, Clone, Default)]
pub struct ObservedField {
    pub value: Option<f32>,
    pub provider: String,
    pub observed_at: String,
    pub fetched_at_s: f64,
    pub quality: DataQuality,
}

impl ObservedField {
    fn update(
        &mut self,
        value: Option<f32>,
        sample: &RealOceanSample,
        fetched_at_s: f64,
        quality: DataQuality,
    ) {
        let Some(value) = value else {
            return;
        };
        self.value = Some(value);
        self.provider = sample.source.clone();
        self.observed_at = sample.timestamp.clone();
        self.fetched_at_s = fetched_at_s;
        self.quality = quality;
    }
}

#[derive(Debug, Clone, Default)]
pub struct EnvironmentalSnapshot {
    pub sst_c: ObservedField,
    pub salinity_psu: ObservedField,
    pub bottom_depth_m: ObservedField,
    pub wave_height_m: ObservedField,
}

#[derive(Resource, Debug, Clone)]
pub struct ApiState {
    pub last_fetch_started_s: f64,
    pub environment: EnvironmentalSnapshot,
    pub recent_samples: Vec<RealOceanSample>,
    pub metrics: ApiMetrics,
    pub provider: ProviderMode,
    pub is_live: bool,
}

impl Default for ApiState {
    fn default() -> Self {
        let mut state = Self {
            last_fetch_started_s: -60.0,
            environment: EnvironmentalSnapshot::default(),
            recent_samples: Vec::new(),
            metrics: ApiMetrics::default(),
            provider: ProviderMode::Hybrid,
            is_live: false,
        };
        let sample = RealOceanSample {
            lat: 46.9,
            lon: -124.1,
            sst_c: Some(11.0),
            salinity_psu: Some(32.5),
            bottom_depth_m: Some(-2500.0),
            wave_height_m: Some(1.5),
            water_temp_c: Some(11.0),
            timestamp: "synthetic-scenario-default".into(),
            source: "Deterministic scenario fallback".into(),
            doi: "none-synthetic".into(),
        };
        state.apply_samples(vec![sample], 0.0, DataQuality::Synthetic);
        state
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum ProviderMode {
    Mock,
    Live,
    #[default]
    Hybrid,
}

#[derive(Debug, Clone, Default)]
pub struct ApiMetrics {
    pub fetches: u32,
    pub failures: u32,
    pub latency_ms_average: f32,
    pub trace_log: Vec<String>,
}

impl ApiState {
    fn log(&mut self, message: impl Into<String>) {
        self.metrics.trace_log.push(message.into());
        if self.metrics.trace_log.len() > 100 {
            self.metrics.trace_log.remove(0);
        }
    }

    fn apply_samples(
        &mut self,
        samples: Vec<RealOceanSample>,
        fetched_at_s: f64,
        quality: DataQuality,
    ) {
        for sample in &samples {
            self.environment
                .sst_c
                .update(sample.sst_c.or(sample.water_temp_c), sample, fetched_at_s, quality);
            self.environment
                .salinity_psu
                .update(sample.salinity_psu, sample, fetched_at_s, quality);
            self.environment
                .bottom_depth_m
                .update(sample.bottom_depth_m, sample, fetched_at_s, quality);
            self.environment
                .wave_height_m
                .update(sample.wave_height_m, sample, fetched_at_s, quality);
        }
        self.recent_samples.extend(samples);
        if self.recent_samples.len() > 32 {
            let excess = self.recent_samples.len() - 32;
            self.recent_samples.drain(0..excess);
        }
        self.is_live = quality == DataQuality::Observed;
    }
}

#[derive(Resource, Debug, Clone)]
pub struct MissionRegion {
    pub lat: f64,
    pub lon: f64,
    pub name: String,
}

impl Default for MissionRegion {
    fn default() -> Self {
        Self {
            // Coarse public demonstration region, not a home or user location.
            lat: 46.9,
            lon: -124.1,
            name: "North Pacific public-data demo region".into(),
        }
    }
}

#[derive(Component)]
struct OceanFetchTask {
    task: Task<Result<Vec<RealOceanSample>, String>>,
    started_at_s: f64,
}

pub struct ApiPlugin;

impl Plugin for ApiPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ApiState::default())
            .insert_resource(MissionRegion::default())
            .add_systems(Startup, setup_api_observability)
            .add_systems(
                Update,
                (fetch_orchestrator_system, poll_fetch_tasks).chain(),
            );
    }
}

fn setup_api_observability(mut api: ResMut<ApiState>) {
    api.log("TRACE: public environmental integration initialized");
    api.log("BOUNDARY: public data changes environment only; targets are synthetic");
    api.log("FALLBACK: deterministic scenario values remain available offline");
}

fn fetch_orchestrator_system(
    mut commands: Commands,
    time: Res<Time>,
    region: Res<MissionRegion>,
    mut api: ResMut<ApiState>,
    active_tasks: Query<Entity, With<OceanFetchTask>>,
) {
    let now = time.elapsed_secs_f64();
    if !active_tasks.is_empty() || now - api.last_fetch_started_s < 60.0 {
        return;
    }

    let latitude = region.lat;
    let longitude = region.lon;
    let task = IoTaskPool::get().spawn(async move {
        let depth = gebco::fetch_depth_wasm(latitude, longitude).await;
        let surface = noaa::fetch_sst_coops_wasm(latitude, longitude).await;
        collect_results(depth, surface)
    });

    commands.spawn(OceanFetchTask {
        task,
        started_at_s: now,
    });
    api.last_fetch_started_s = now;
    api.metrics.fetches += 1;
    api.log(format!(
        "FETCH_STARTED: region={} lat={latitude:.1} lon={longitude:.1}",
        region.name
    ));
}

fn poll_fetch_tasks(
    mut commands: Commands,
    time: Res<Time>,
    mut api: ResMut<ApiState>,
    mut tasks: Query<(Entity, &mut OceanFetchTask)>,
) {
    for (entity, mut fetch_task) in tasks.iter_mut() {
        let Some(result) = future::block_on(future::poll_once(&mut fetch_task.task)) else {
            continue;
        };

        let now = time.elapsed_secs_f64();
        let latency_ms = ((now - fetch_task.started_at_s) * 1000.0).max(0.0) as f32;
        let fetch_count = api.metrics.fetches.max(1) as f32;
        api.metrics.latency_ms_average +=
            (latency_ms - api.metrics.latency_ms_average) / fetch_count;

        match result {
            Ok(samples) => {
                let quality = if cfg!(target_arch = "wasm32") {
                    DataQuality::Observed
                } else {
                    DataQuality::Synthetic
                };
                api.apply_samples(samples, now, quality);
                api.log(format!("FETCH_APPLIED: latency_ms={latency_ms:.1}"));
            }
            Err(error) => {
                api.metrics.failures += 1;
                api.is_live = false;
                api.log(format!("FETCH_FAILED: {error}"));
            }
        }
        commands.entity(entity).despawn();
    }
}

fn collect_results(
    depth: Result<RealOceanSample, String>,
    surface: Result<RealOceanSample, String>,
) -> Result<Vec<RealOceanSample>, String> {
    let mut samples = Vec::new();
    let mut errors = Vec::new();

    match depth {
        Ok(sample) => samples.push(sample),
        Err(error) => errors.push(format!("depth: {error}")),
    }
    match surface {
        Ok(sample) => samples.push(sample),
        Err(error) => errors.push(format!("surface: {error}")),
    }

    if samples.is_empty() {
        Err(errors.join("; "))
    } else {
        Ok(samples)
    }
}

/// Provider abstraction retained for backend and test implementations.
pub trait OceanDataProvider: Send + Sync {
    fn fetch(&self, lat: f64, lon: f64) -> Task<Result<RealOceanSample, String>>;
    fn name(&self) -> &'static str;
}

pub struct MockProvider;

impl OceanDataProvider for MockProvider {
    fn name(&self) -> &'static str {
        "DeterministicMockProvider"
    }

    fn fetch(&self, lat: f64, lon: f64) -> Task<Result<RealOceanSample, String>> {
        IoTaskPool::get().spawn(async move {
            Ok(RealOceanSample {
                lat,
                lon,
                sst_c: Some(11.0),
                salinity_psu: Some(32.5),
                bottom_depth_m: Some(-2500.0),
                wave_height_m: Some(1.5),
                water_temp_c: Some(11.0),
                timestamp: "synthetic-scenario-default".into(),
                source: "Deterministic scenario fallback".into(),
                doi: "none-synthetic".into(),
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_fields_do_not_overwrite_unrelated_measurements() {
        let mut state = ApiState::default();
        let depth_provider = RealOceanSample {
            bottom_depth_m: Some(-1200.0),
            source: "depth-provider".into(),
            timestamp: "depth-time".into(),
            ..Default::default()
        };
        let surface_provider = RealOceanSample {
            sst_c: Some(12.0),
            source: "surface-provider".into(),
            timestamp: "surface-time".into(),
            ..Default::default()
        };
        state.apply_samples(
            vec![depth_provider, surface_provider],
            10.0,
            DataQuality::Observed,
        );
        assert_eq!(state.environment.bottom_depth_m.provider, "depth-provider");
        assert_eq!(state.environment.sst_c.provider, "surface-provider");
    }
}

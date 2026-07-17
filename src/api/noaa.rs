//! Minimal NOAA CO-OPS environmental adapter.
//! Public observations influence the environment only, never synthetic tracks.

use super::RealOceanSample;

const DEMO_STATION: &str = "9439040";

#[cfg(target_arch = "wasm32")]
pub async fn fetch_sst_coops_wasm(lat: f64, lon: f64) -> Result<RealOceanSample, String> {
    use gloo_net::http::Request;

    let url = format!(
        "https://api.tidesandcurrents.noaa.gov/api/prod/datagetter?date=latest&station={DEMO_STATION}&product=water_temperature&datum=STND&time_zone=gmt&units=metric&format=json"
    );
    let response = Request::get(&url)
        .send()
        .await
        .map_err(|error| format!("NOAA CO-OPS request failed: {error:?}"))?;
    if !response.ok() {
        return Err(format!("NOAA CO-OPS returned HTTP {}", response.status()));
    }

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|error| format!("NOAA CO-OPS JSON failed: {error:?}"))?;
    let data = json["data"]
        .as_array()
        .ok_or_else(|| "NOAA CO-OPS response contained no data array".to_string())?;
    let latest = data
        .last()
        .ok_or_else(|| "NOAA CO-OPS response contained no observations".to_string())?;
    let temperature = latest["v"]
        .as_str()
        .and_then(|value| value.parse::<f32>().ok())
        .ok_or_else(|| "NOAA CO-OPS temperature was missing or invalid".to_string())?;
    let observed_at = latest["t"].as_str().unwrap_or("unknown").to_string();

    Ok(RealOceanSample {
        lat,
        lon,
        sst_c: Some(temperature),
        salinity_psu: None,
        bottom_depth_m: None,
        wave_height_m: None,
        water_temp_c: Some(temperature),
        timestamp: observed_at,
        source: format!("NOAA CO-OPS station {DEMO_STATION}"),
        doi: "NOAA Tides and Currents API".into(),
    })
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn fetch_sst_coops_wasm(lat: f64, lon: f64) -> Result<RealOceanSample, String> {
    Ok(RealOceanSample {
        lat,
        lon,
        sst_c: Some(10.5),
        salinity_psu: Some(34.8),
        bottom_depth_m: None,
        wave_height_m: Some(2.1),
        water_temp_c: Some(10.5),
        timestamp: "synthetic-native-fixture".into(),
        source: "Deterministic NOAA-shaped fixture".into(),
        doi: "none-synthetic".into(),
    })
}

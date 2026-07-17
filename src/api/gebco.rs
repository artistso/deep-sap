//! Minimal GEBCO bathymetry adapter through OpenTopoData.
//! GEBCO data is contextual and must not be used for navigation.

use super::RealOceanSample;

pub fn opentopo_gebco_url(lat: f64, lon: f64) -> String {
    format!("https://api.opentopodata.org/v1/gebco2020?locations={lat},{lon}")
}

#[cfg(target_arch = "wasm32")]
pub async fn fetch_depth_wasm(lat: f64, lon: f64) -> Result<RealOceanSample, String> {
    use gloo_net::http::Request;

    let response = Request::get(&opentopo_gebco_url(lat, lon))
        .send()
        .await
        .map_err(|error| format!("GEBCO request failed: {error:?}"))?;
    if !response.ok() {
        return Err(format!("GEBCO provider returned HTTP {}", response.status()));
    }

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|error| format!("GEBCO JSON failed: {error:?}"))?;
    let elevation = json["results"]
        .as_array()
        .and_then(|results| results.first())
        .and_then(|result| result["elevation"].as_f64())
        .ok_or_else(|| "GEBCO provider returned no elevation".to_string())? as f32;

    Ok(RealOceanSample {
        lat,
        lon,
        sst_c: None,
        salinity_psu: None,
        bottom_depth_m: Some(elevation),
        wave_height_m: None,
        water_temp_c: None,
        timestamp: "provider-latest".into(),
        source: "GEBCO 2020 via OpenTopoData".into(),
        doi: "GEBCO grid; see provider attribution".into(),
    })
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn fetch_depth_wasm(lat: f64, lon: f64) -> Result<RealOceanSample, String> {
    Ok(RealOceanSample {
        lat,
        lon,
        sst_c: None,
        salinity_psu: None,
        bottom_depth_m: Some(-2_850.0),
        wave_height_m: None,
        water_temp_c: None,
        timestamp: "synthetic-native-fixture".into(),
        source: "Deterministic GEBCO-shaped fixture".into(),
        doi: "none-synthetic".into(),
    })
}

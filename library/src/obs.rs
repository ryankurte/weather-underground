

use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

/// Object that represents an observation with imperial or metric values
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObservationValue {
    pub dewpt: Option<f64>,
    pub elev: Option<f64>,
    pub heat_index: Option<f64>,
    pub precip_rate: Option<f64>,
    pub precip_total: Option<f64>,
    pub pressure: Option<f64>,
    pub temp: Option<f64>,
    pub wind_chill: Option<f64>,
    pub wind_gust: Option<f64>,
    pub wind_speed: Option<f64>,
}

/// Object that represents an observation
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Observation {
    pub country: Option<String>,
    pub epoch: u64,
    pub humidity: Option<f64>,
    pub lat: f64,
    pub lon: f64,
    pub imperial: Option<ObservationValue>,
    pub metric: Option<ObservationValue>,
    pub neighborhood: Option<String>,
    pub obs_time_local: String,
    pub obs_time_utc: String,
    pub solar_radiation: Option<f64>,
    pub uv: Option<f64>,
    pub winddir: Option<f64>,
}

impl Observation {
    pub fn values(&self) -> Option<&ObservationValue> {
        self.metric.as_ref().or(self.imperial.as_ref())
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ObservationError {
    pub code: String,
    pub message: String,
}

/// Object returned by the weather underground API
#[derive(Debug, Deserialize, Serialize)]
pub struct ObservationResponse {
    pub errors: Option<Vec<ObservationError>>,
    pub observations: Option<Vec<Observation>>,
    pub metadata: Option<serde_json::Value>,
    pub success: Option<bool>,
}

impl ObservationResponse {
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(&self.observations)
    }
}

impl TryFrom<serde_json::Value> for ObservationResponse {
    type Error = serde_json::Error;

    fn try_from(value: serde_json::Value) -> Result<Self, Self::Error> {
        serde_json::from_value(value)
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct HistoryResponse {
    pub observations: Vec<Observation>,
}

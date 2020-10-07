#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use std::convert::TryFrom;
use std::str::FromStr;

lazy_static! {
    static ref API_KEY_REGEX: Regex = Regex::new(r"apiKey=([a-z0-9]+)").unwrap();
}

#[derive(Debug)]
pub struct ParseUnitError;

impl ToString for ParseUnitError {
    fn to_string(&self) -> String {
        String::from("Invalid unit value")
    }
}

/// Representation for the unit type that will be used in the response
#[derive(Debug)]
pub enum Unit {
    English,
    Metric,
}

impl Unit {
    fn as_str(&self) -> &str {
        match self {
            Self::English => "e",
            Self::Metric => "m",
        }
    }
}

impl FromStr for Unit {
    type Err = ParseUnitError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "e" => Ok(Self::English),
            "m" => Ok(Self::Metric),
            _ => Err(ParseUnitError),
        }
    }
}

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
    pub country: String,
    pub epoch: u64,
    pub humidity: Option<f64>,
    pub lat: f64,
    pub lon: f64,
    pub imperial: Option<ObservationValue>,
    pub metric: Option<ObservationValue>,
    pub neighborhood: String,
    pub obs_time_local: String,
    pub obs_time_utc: String,
    pub solar_radiation: Option<f64>,
    pub uv: Option<f64>,
    pub winddir: Option<f64>,
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

/// All the types of error for the library
#[derive(Debug)]
pub enum Error {
    ApiKeyNotFound,
    ApiKeyInvalid,
    Reqwest(reqwest::Error),
    PayloadInvalid(serde_json::Error),
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Self::Reqwest(err)
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::PayloadInvalid(err)
    }
}

fn parse_api_key(html: &str) -> Result<String, Error> {
    match API_KEY_REGEX
        .find(html)
        .map(|found| String::from(found.as_str()))
        .map(|found| found[7..].into())
    {
        Some(value) => Ok(value),
        None => Err(Error::ApiKeyNotFound),
    }
}

/// Create a reqwest client that will be used to make the requests
pub fn create_client(timeout: Duration) -> Result<reqwest::Client, reqwest::Error> {
    debug!("creating client with timeout {:?}", timeout);
    reqwest::Client::builder()
        .cookie_store(true)
        .gzip(true)
        .timeout(timeout)
        .http2_prior_knowledge()
        .use_rustls_tls()
        .build()
}

/// Fetch the wunderground homepage and parse an api token
/// 
/// ```
/// use std::time::Duration;
/// use weather_underground as wu;
/// async {
///     let client = wu::create_client(Duration::from_secs(2)).unwrap();
///     let api_key = wu::fetch_api_key(&client).await.unwrap();
///     println!("key: {}", api_key);
/// };
/// ```
pub async fn fetch_api_key(client: &reqwest::Client) -> Result<String, Error> {
    debug!("fetching new api key");
    let html = client
        .get("https://www.wunderground.com")
        .send()
        .await?
        .text()
        .await?;
    parse_api_key(html.as_str())
}

/// Fetch observations from the weatherunderground api
/// 
/// ```
/// use std::convert::TryFrom;
/// use std::time::Duration;
/// use weather_underground as wu;
/// async {
///     let client = wu::create_client(Duration::from_secs(2)).unwrap();
///     let api_key = wu::fetch_api_key(&client).await.unwrap();
///     let unit = wu::Unit::Metric;
///     let result = wu::fetch_observation(&client, api_key.as_str(), "IPARIS18204", &unit).await.unwrap();
///     if let Some(response) = result {
///         let response = wu::ObservationResponse::try_from(response).unwrap();
///         println!("response: {:?}", response);
///     } else {
///         println!("no data from server");
///     }
/// };
/// ```
pub async fn fetch_observation(
    client: &reqwest::Client,
    api_key: &str,
    station_id: &str,
    unit: &Unit,
) -> Result<Option<serde_json::Value>, Error> {
    debug!("fetching observation for station {}", station_id);
    let url = format!("https://api.weather.com/v2/pws/observations/current?apiKey={}&stationId={}&numericPrecision=decimal&format=json&units={}", api_key, station_id, unit.as_str());
    let response = client
        .get(url.as_str())
        .header("Accept-Encoding", "gzip")
        .send()
        .await?;
    if response.status().as_u16() == 204 {
        return Ok(None);
    }
    let body = response.json().await?;
    Ok(Some(body))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parsing_existing_api_key() {
        let page = include_str!("../test/home.html");
        let result = parse_api_key(page);
        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result, "6532d6454b8aa370768e63d6ba5a832e");
    }

    #[test]
    fn parsing_missing_api_key() {
        let result = parse_api_key("whatever");
        assert!(result.is_err());
        let result = result.unwrap_err();
        assert!(matches!(result, Error::ApiKeyNotFound));
    }

    #[test]
    fn parsing_observations() {
        let result = include_str!("../test/result.json");
        let result: serde_json::Value = serde_json::from_str(result).unwrap();
        let result = ObservationResponse::try_from(result);
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.observations.is_some());
        let result = result.observations.unwrap();
        assert_eq!(result.len(), 1);
    }
}
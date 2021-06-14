#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use std::convert::TryFrom;

use strum_macros::{Display, EnumString};

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

#[derive(Debug, Clone, PartialEq, Display, EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum Format {
    Json,
}

#[derive(Debug, Clone, PartialEq, Display, EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum History {
    Current,
    Hourly,
    Daily,
    All,
}

#[derive(Debug, Clone, PartialEq, Display, EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum Precision {
    Integer,
    Decimal,
}

#[derive(Debug, Clone, PartialEq, Display, EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum Unit {
    #[strum(serialize="e")]
    Imperial,
    #[strum(serialize="m")]
    Metric,
}


pub struct ObservationArgs {
    pub format: Format,
    pub unit: Unit,
    pub history: History,
    pub precision: Precision,
}

impl Default for ObservationArgs {
    fn default() -> Self {
        Self{
            unit: Unit::Metric,
            format: Format::Json,
            history: History::Current,
            precision: Precision::Decimal,
        }
    }
}

impl ObservationArgs {
    pub fn build_query(&self, api_key: &str, station_id: &str) -> String {
        let mut base = self.history.to_string();

        base += &format!("?apiKey={}", api_key);

        base += &format!("&stationId={}", station_id);

        base += &format!("&units={}", self.unit);

        base += &format!("&format={}", self.format);

        if self.precision == Precision::Decimal {
            base += "&numericPrecision=decimal"
        }

        base
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

#[derive(Debug)]
pub struct Client {
    api_key: String,
    c: reqwest::Client,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClientOpts {
    pub timeout: Duration,
}

impl Default for ClientOpts {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(2),
        }
    }
}

impl Client {
    /// Create a new wunderground API client
    pub async fn create(api_key: Option<&str>, opts: ClientOpts) -> Result<Self, Error> {
        debug!("creating client with options {:?}", opts);
        let c = reqwest::Client::builder()
            .cookie_store(true)
            .gzip(true)
            .timeout(opts.timeout)
            .http2_prior_knowledge()
            .use_rustls_tls()
            .build()?;

        // Fetch API key from web if not provided
        let api_key = match api_key {
            Some(s) => s.to_string(),
            None => {
                let html = c
                    .get("https://www.wunderground.com")
                    .send()
                    .await?
                    .text()
                    .await?;
                parse_api_key(html.as_str())?
            }
        };

        Ok(Self{api_key, c})
    }

    /// Access internal reqwest client
    pub fn inner(&mut self) -> &mut reqwest::Client {
        &mut self.c
    }

    /// Request an observation
    pub async fn fetch_observation_raw(
        &mut self,
        station_id: &str,
        opts: &ObservationArgs,
    ) -> Result<Option<serde_json::Value>, Error> {
        debug!("fetching observation for station {}", station_id);

        let url = format!("https://api.weather.com/v2/pws/observations/{}", opts.build_query(&self.api_key, station_id));
    
        let response = self.c
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
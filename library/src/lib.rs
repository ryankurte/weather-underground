use std::time::Duration;
use std::convert::TryFrom;

use log::{trace, debug};
use obs::HistoryResponse;
use regex::Regex;
use reqwest::StatusCode;
use strum_macros::{Display, EnumString};

pub use chrono::{naive::NaiveDate, Datelike};


mod obs;
pub use obs::{Observation, ObservationResponse, ObservationValue, ObservationError};

lazy_static::lazy_static! {
    static ref API_KEY_REGEX: Regex = Regex::new(r"apiKey=([a-z0-9]+)").unwrap();
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


/// Arguments for observation requests
#[derive(Debug, PartialEq, Clone)]
pub struct RequestOpts {
    pub format: Format,
    pub unit: Unit,
    pub precision: Precision,
    pub history: History,
    pub date: Option<NaiveDate>,
}

impl Default for RequestOpts {
    fn default() -> Self {
        Self{
            unit: Unit::Metric,
            format: Format::Json,
            history: History::Current,
            precision: Precision::Decimal,
            date: None,
        }
    }
}

impl RequestOpts {
    pub fn build_query(&self, api_key: &str, station_id: &str) -> String {
        let mut base = match self.history {
            History::Current => "observations/".to_string(),
            _ => "history/".to_string(),
        };

        base += &self.history.to_string();

        base += &format!("?apiKey={}", api_key);

        base += &format!("&stationId={}", station_id);

        base += &format!("&units={}", self.unit);

        base += &format!("&format={}", self.format);

        if self.precision == Precision::Decimal {
            base += "&numericPrecision=decimal"
        }

        if let Some(d) = &self.date {
            base += &format!("&date={:04}{:02}{:02}", d.year(), d.month(), d.day());
        }

        base
    }
}

/// All the types of error for the library
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("API key not found")]
    ApiKeyNotFound,
    #[error("API key invalid")]
    ApiKeyInvalid,
    #[error("Reqwest error: {0}")]
    Reqwest(reqwest::Error),
    #[error("JSON error: {0}")]
    PayloadInvalid(serde_json::Error),
    #[error("HTTP error: {0}")]
    Http(StatusCode),
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


/// wunderground.com API client
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

    /// Request an observation, returning raw JSON data
    pub async fn request(
        &mut self,
        station_id: &str,
        opts: &RequestOpts,
    ) -> Result<Option<serde_json::Value>, Error> {
        debug!("fetching observation(s) for station {}", station_id);

        let url = format!("https://api.weather.com/v2/pws/{}", opts.build_query(&self.api_key, station_id));
    
        let response = self.c
            .get(url.as_str())
            .header("Accept-Encoding", "gzip")
            .send()
            .await?;

        if response.status().as_u16() == 204 {
            return Ok(None);
        } else if response.status().as_u16() != 200 {
            return Err(Error::Http(response.status()))
        }

        let body = response.json().await?;

        trace!("Response: {:?}", body);

        Ok(Some(body))
    }

    pub async fn fetch_current(
        &mut self,
        station_id: &str,
        opts: &RequestOpts,
    ) -> Result<Option<ObservationResponse>, Error> {
        let raw = match self.request(station_id, opts).await? {
            Some(r) => r,
            None => return Ok(None),
        };

        let response = ObservationResponse::try_from(raw)?;

        Ok(Some(response))
    }

    pub async fn fetch_history(
        &mut self,
        station_id: &str,
        opts: &RequestOpts,
    ) -> Result<Option<HistoryResponse>, Error> {
        let raw = match self.request(station_id, opts).await? {
            Some(r) => r,
            None => return Ok(None),
        };
        let response = serde_json::from_value(raw)?;

        Ok(Some(response))
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
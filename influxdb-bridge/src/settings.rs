use std::time::Duration;
use weather_underground as wu;

#[derive(Clone)]
pub struct Settings {
    pub stations: Vec<String>,
    pub timeout: Duration,
    pub interval: Duration,
    pub unit: wu::Unit,
    pub influxdb_host: String,
    pub influxdb_username: String,
    pub influxdb_password: String,
    pub influxdb_database: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            stations: Self::read_stations(),
            timeout: Self::read_timeout(),
            interval: Self::read_interval(),
            unit: wu::Unit::Metric,
            influxdb_host: std::env::var("INFLUX_HOST")
                .unwrap_or_else(|_| "http://localhost:8086".into()),
            influxdb_username: std::env::var("INFLUX_USERNAME")
                .unwrap_or_else(|_| "username".into()),
            influxdb_password: std::env::var("INFLUX_PASSWORD")
                .unwrap_or_else(|_| "password".into()),
            influxdb_database: std::env::var("INFLUX_DATABASE")
                .unwrap_or_else(|_| "default".into()),
        }
    }
}

impl Settings {
    fn read_interval() -> Duration {
        let value = match std::env::var("WU_INTERVAL") {
            Ok(value) => value,
            Err(_) => return Duration::from_secs(60),
        };
        match value.parse::<u64>() {
            Ok(value) => Duration::from_millis(value),
            Err(_) => panic!("unable to parse WU_INTERVAL"),
        }
    }

    fn read_stations() -> Vec<String> {
        let value = match std::env::var("WU_STATIONS") {
            Ok(value) => value.split(",").map(|v| v.into()).collect::<Vec<String>>(),
            Err(_) => panic!("unable to parse WU_STATIONS"),
        };
        if value.is_empty() {
            panic!("WU_STATIONS shouldn't be empty")
        } else {
            value
        }
    }

    fn read_timeout() -> Duration {
        let value = match std::env::var("WU_TIMEOUT") {
            Ok(value) => value,
            Err(_) => return Duration::from_secs(10),
        };
        match value.parse::<u64>() {
            Ok(value) => Duration::from_millis(value),
            Err(_) => panic!("unable to parse WU_TIMEOUT"),
        }
    }
}

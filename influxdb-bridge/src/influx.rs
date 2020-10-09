use crate::settings::Settings;
use influx_db_client as influxdb;
use std::str::FromStr;
use weather_underground as wu;

macro_rules! add_float {
    ($origin:expr, $dest:expr, $key:ident, $target:expr) => {
        if let Some(value) = $origin.$key {
            $dest.add_field($target, influxdb::Value::Float(value))
        } else {
            $dest
        }
    };
    ($origin:expr, $dest:expr, $key:ident) => {
        add_float!($origin, $dest, $key, stringify!($key))
    };
}

pub async fn publish(settings: &Settings, station_id: &str, response: &wu::ObservationResponse) {
    debug!("publishing for station {}", station_id);
    let url = reqwest::Url::from_str(settings.influxdb_host.as_str()).unwrap();
    let client = influxdb::Client::new(url, settings.influxdb_database.clone())
        .set_authentication(settings.influxdb_username.as_str(), settings.influxdb_password.as_str());
    let name = format!("weather-underground_{}", station_id);
    if let Some(observations) = response.observations.as_ref() {
        for obs in observations.iter() {
            let point = influxdb::Point::new(name.as_str());
            let point = point
                .add_tag("unit", influxdb::Value::String(settings.unit.as_str().into()))
                .add_tag("country", influxdb::Value::String(obs.country.clone()))
                .add_tag("neighborhood", influxdb::Value::String(obs.neighborhood.clone()))
                .add_tag("lat", influxdb::Value::Float(obs.lat))
                .add_tag("lng", influxdb::Value::Float(obs.lon));
            let point = add_float!(obs, point, humidity);
            let point = add_float!(obs, point, solar_radiation);
            let point = add_float!(obs, point, uv);
            let point = add_float!(obs, point, winddir, "wind_dir");
            let point = match obs.values() {
                Some(values) => {
                    let point = add_float!(values, point, dewpt);
                    let point = add_float!(values, point, elev);
                    let point = add_float!(values, point, heat_index);
                    let point = add_float!(values, point, precip_rate);
                    let point = add_float!(values, point, precip_total);
                    let point = add_float!(values, point, pressure);
                    let point = add_float!(values, point, temp);
                    let point = add_float!(values, point, wind_chill);
                    let point = add_float!(values, point, wind_gust);
                    add_float!(values, point, wind_speed)
                },
                None => point,
            };
            match client.write_point(point, None, None).await {
                Ok(_) => info!("published for {}", station_id),
                Err(err) => error!("error: {}", err),
            };
        }
    }
}

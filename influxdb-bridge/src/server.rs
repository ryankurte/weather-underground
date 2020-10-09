use crate::influx;
use crate::settings::Settings;
use reqwest;
use std::convert::TryFrom;
use tokio::time::delay_for;
use weather_underground as wu;

#[derive(Debug)]
pub enum Error {
    ApiKeyNotFound,
    TooManyRetry,
}

#[derive(Default)]
struct Server {
    api_key: String,
}

impl Server {
    async fn get_api_key(&mut self, client: &reqwest::Client) -> Result<&str, Error> {
        if self.api_key.is_empty() {
            self.api_key = match wu::fetch_api_key(client).await {
                Ok(value) => value,
                Err(_) => return Err(Error::ApiKeyNotFound),
            };
        }
        Ok(self.api_key.as_str())
    }

    async fn process(
        &mut self,
        client: &reqwest::Client,
        settings: &Settings,
        station_id: &str,
        retry: usize,
    ) -> Result<(), Error> {
        debug!("processing station {}", station_id);
        let api_key = self.get_api_key(client).await?;
        for _idx in (0..retry).rev() {
            let result =
                match wu::fetch_observation(client, api_key, station_id, &settings.unit).await {
                    Err(err) => {
                        error!("couldn't fetch observation: {:?}", err);
                        continue;
                    }
                    Ok(value) => value,
                };
            let result = match result {
                Some(value) => value,
                None => return Ok(()),
            };
            let result = match wu::ObservationResponse::try_from(result) {
                Ok(value) => value,
                Err(err) => {
                    error!("unable to parse response: {}", err);
                    return Ok(());
                }
            };

            influx::publish(settings, station_id, &result).await;
            return Ok(())
        }
        info!("processing station {} success", station_id);
        Err(Error::TooManyRetry)
    }

    async fn iterate(
        &mut self,
        client: &reqwest::Client,
        settings: &Settings,
    ) -> Result<(), Error> {
        debug!("iteration");
        for station_id in settings.stations.iter() {
            self.process(client, settings, station_id.as_str(), 10)
                .await?;
        }
        info!("iteration done");
        Ok(())
    }

    async fn sleep(&self, settings: &Settings) {
        debug!("sleeping for {:?}", settings.interval);
        delay_for(settings.interval).await;
    }

    pub async fn run(
        &mut self,
        client: &reqwest::Client,
        settings: &Settings,
    ) -> Result<(), Error> {
        loop {
            self.iterate(client, settings).await?;
            self.sleep(settings).await;
        }
    }
}

pub async fn run(settings: &Settings) {
    let client = wu::create_client(settings.timeout).expect("unable to create client");
    let mut srv = Server::default();
    srv.run(&client, settings)
        .await
        .expect("something happened");
}

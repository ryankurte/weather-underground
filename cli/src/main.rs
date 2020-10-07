use clap::{crate_version, Clap};
use std::time::Duration;
use weather_underground as wu;
use std::convert::TryFrom;

#[derive(Clap, Debug)]
#[clap(
    version = crate_version!(),
    author = "Jeremie Drouet <jeremie.drouet@gmail.com>"
)]
struct Options {
    #[clap(short, long, about = "Timeout in ms", default_value = "10000")]
    pub timeout: u64,
    #[clap(short, long, about = "Unit (m for metric, e for imperial)", default_value = "m")]
    pub unit: wu::Unit,
    #[clap(about = "ID of the station you want the observations from")]
    pub station_id: String,
}

impl Options {
    pub fn get_timeout(&self) -> Duration {
        Duration::from_millis(self.timeout)
    }
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let opts = Options::parse();

    let client = wu::create_client(opts.get_timeout()).expect("couldn't create client");
    let api_key = wu::fetch_api_key(&client).await.expect("couldn't fetch api key");
    let result = wu::fetch_observation(&client, api_key.as_str(), opts.station_id.as_str(), &opts.unit).await.expect("couldn't fetch observation");
    if let Some(value) = result {
        let result = wu::ObservationResponse::try_from(value).expect("couldn't parse response");
        println!("{}", result.to_json().expect("couldn't format observations"));
    } else {
        eprintln!("no result...");
    }
}
       
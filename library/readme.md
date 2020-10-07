# Weather underground library

A simple library to parse responses from weather underground api.

## Installation

```toml
[dependencies]
weather-underground = "0.1"
```

## Usage

```rust
use std::convert::TryFrom;
use std::time::Duration;
use weather_underground as wu;

async {
    let client = wu::create_client(Duration::from_secs(2)).unwrap();
    let api_key = wu::fetch_api_key(&client).await.unwrap();
    let unit = wu::Unit::Metric;
    let result = wu::fetch_observation(&client, api_key.as_str(), "IPARIS18204", &unit).await.unwrap();
    if let Some(response) = result {
        let response = wu::ObservationResponse::try_from(response).unwrap();
        println!("response: {:?}", response);
    } else {
        println!("no data from server");
    }
};
```

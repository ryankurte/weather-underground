#[macro_use] extern crate log;

mod influx;
mod server;
mod settings;

#[tokio::main]
async fn main() {
    env_logger::init();

    let settings = settings::Settings::default();
    server::run(&settings).await;
}

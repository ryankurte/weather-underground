FROM rust:1-slim-buster AS builder

RUN apt-get update \
  && apt-get install -y libssl-dev pkg-config \
  && rm -rf /var/lib/apt/lists/*

ENV USER=root

COPY Cargo.toml /code/Cargo.toml
COPY cli /code/cli
COPY library /code/library
COPY influxdb-bridge /code/influxdb-bridge

WORKDIR /code/influxdb-bridge

RUN cargo build --release

FROM rust:1-slim-buster

RUN apt-get update \
  && apt-get install -y ca-certificates libssl1.1 \
  && rm -rf /var/lib/apt/lists/*

COPY --from=builder /code/target/release/weather-underground-influxdb-bridge /weather-underground-influxdb-bridge

CMD ["./weather-underground-influxdb-bridge"]

FROM rust:1.96.1-bookworm AS build
RUN apt-get update && apt-get install -y --no-install-recommends python3 libasound2-dev libudev-dev libv4l-dev libclang-dev clang pkg-config libpipewire-0.3-dev libxcb-randr0-dev libdbus-1-dev libgbm-dev libegl1-mesa-dev && rm -rf /var/lib/apt/lists/*
RUN rustup target add wasm32-unknown-unknown && cargo install --locked wasm-bindgen-cli --version 0.2.130
WORKDIR /source
COPY . .
RUN mkdir -p data && python3 tools/build_web.py && cargo build --release --bin commons-desktop

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates libasound2 libudev1 libv4l-0 libxkbcommon0 libpipewire-0.3-0 libxcb-randr0 libdbus-1-3 libgbm1 libegl1 && rm -rf /var/lib/apt/lists/* && useradd --create-home --uid 10001 commons && mkdir /data && chown commons:commons /data
COPY --from=build /source/target/release/commons-desktop /usr/local/bin/commons
USER commons
ENV COMMONS_DATA_DIR=/data
EXPOSE 8080
ENTRYPOINT ["commons", "--serve-web", "--http-bind", "0.0.0.0:8080", "--tcp-bind", "127.0.0.1:7777"]

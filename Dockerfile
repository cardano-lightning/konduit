FROM rust:1.90-bookworm AS build
WORKDIR /dist
COPY . .
WORKDIR rust
RUN cargo build --release -p konduit-server

FROM debian:bookworm-slim AS run
RUN apt-get update \
  && apt-get install -y --no-install-recommends ca-certificates libssl3 curl \
  && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=build /dist/rust/target/release/konduit-server /app/konduit-server
EXPOSE 4444
CMD ["/app/konduit-server"]

FROM rust:slim as chef

RUN rustup toolchain install nightly
RUN rustup default nightly

RUN apt-get update -y && apt-get install build-essential lld clang -y && rm -rf /var/lib/apt/lists/*

RUN cargo install cargo-chef

FROM chef AS planner

WORKDIR /planner
COPY . /planner
RUN cargo chef prepare --recipe-path recipe.json

FROM chef as builder

WORKDIR /builder/

COPY --from=planner planner/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

COPY . /builder/
RUN cargo build --release -p pjatk-api

FROM debian:bullseye-slim AS runner

WORKDIR /usr/local/bin/

COPY --from=builder builder/target/release/pjatk-api /usr/local/bin/

RUN rm -rf /var/lib/apt/lists/*

ENTRYPOINT ["pjatk-api"]
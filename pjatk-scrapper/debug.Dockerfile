FROM rust:slim as chef
# nightly-2022-03-22
RUN rustup toolchain install stable
RUN rustup default stable

RUN apt-get update -y && apt-get install build-essential pkg-config lld clang -y && rm -rf /var/lib/apt/lists/*

RUN cargo install cargo-chef

FROM chef AS planner

WORKDIR /planner
COPY . /planner
RUN cargo chef prepare --recipe-path recipe.json

FROM chef as builder

WORKDIR /builder/

COPY --from=planner planner/recipe.json recipe.json
RUN cargo chef cook --recipe-path recipe.json

COPY . /builder/
RUN cargo build -p pjatk-scrapper

FROM debian:bullseye-slim AS runner

WORKDIR /usr/local/bin/

COPY --from=builder builder/target/debug/pjatk-scrapper /usr/local/bin/

RUN rm -rf /var/lib/apt/lists/*

ENTRYPOINT ["pjatk-scrapper"]

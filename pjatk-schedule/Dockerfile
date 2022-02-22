FROM rust:slim as chef

RUN apt-get update -y && apt-get install build-essential -y && rm -rf /var/lib/apt/lists/*

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
RUN cargo build --release -p pjatk-schedule

FROM ubuntu:latest AS runner

WORKDIR /usr/local/bin/

COPY --from=builder builder/target/release/pjatk-schedule /usr/local/bin/

RUN apt-get update -y && apt-get install ca-certificates no-install-recommends -y && rm -rf /var/lib/apt/lists/*

ENTRYPOINT ["pjatk-schedule"]
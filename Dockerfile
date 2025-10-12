# syntax=docker/dockerfile:1

FROM rust:1.90.0-alpine3.22 AS planner
RUN apk update && \
    apk add --no-cache musl-dev && \
    cargo install cargo-chef --locked

WORKDIR /app
# Copy the whole project
COPY . .
# Prepare a build plan ("recipe")
RUN cargo chef prepare --recipe-path recipe.json

FROM rust:1.90.0-alpine3.22 AS builder
RUN apk update && \
    apk add --no-cache musl-dev openssl-dev openssl-libs-static && \
    cargo install cargo-chef --locked

WORKDIR /app
# Copy the build plan from the previous Docker stage
COPY --from=planner /app/recipe.json /app/recipe.json

# Build dependencies - this layer is cached as long as `recipe.json`
# doesn't change.
RUN cargo chef cook --recipe-path recipe.json

# Build the whole project
COPY . .
RUN cargo build --release

FROM alpine:3.22

COPY --from=builder --chown=1000:1000 /app/target/release/trunk-processor /trunk-processor
USER 1000:1000
CMD ["/trunk-processor"]
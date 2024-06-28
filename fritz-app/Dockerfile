# Build the project
FROM rust:1.79-bookworm as builder

# Create a dummy project to cache dependencies
WORKDIR /fritz-log-parser/

RUN cargo new fritz-app --bin
RUN cargo new fritz-api --lib

# Copy the dependencies and build to cache them
COPY ./Cargo.toml ./Cargo.lock ./
COPY ./fritz-api/Cargo.toml ./fritz-api/
COPY ./fritz-app/Cargo.toml ./fritz-app/
RUN cargo build --release

# Copy necessary files to build the actual project
COPY ./ ./

# Prevent some caching thing idk
RUN touch ./fritz-api/src/lib.rs && \
    touch ./fritz-app/src/main.rs

# Build the actual project
ENV SQLX_OFFLINE=true
RUN cargo build --release --bin fritz-app

# Run the built binary
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y sqlite3

COPY --from=builder /fritz-log-parser/target/release/fritz-app /usr/local/bin/
CMD ["/usr/local/bin/fritz-app"]

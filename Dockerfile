# Build the project
FROM rust:1.72-bookworm as builder

# Create a dummy project to cache dependencies
RUN cargo new fritz-log-parser --bin
WORKDIR /fritz-log-parser/

# Copy the dependencies and build to cache them
COPY ./Cargo.toml ./Cargo.lock ./
RUN cargo build --release

# Copy necessary files to build the actual project
COPY ./ ./
# Prevent some caching thing idk
RUN touch ./src/main.rs
# Build the actual project
RUN cargo build --release --bin fetch_logs_regular

# Run the built binary
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y sqlite3

COPY --from=builder /fritz-log-parser/target/release/fetch_logs_regular /usr/local/bin/
CMD ["/usr/local/bin/fetch_logs_regular"]

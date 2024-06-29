# Build the project
FROM rust:1.79-bookworm as builder

# Create a dummy project to cache dependencies
RUN cargo new fritz-app --bin
WORKDIR /fritz-app/

# Copy the dependencies and build to cache them
COPY ./Cargo.toml ./Cargo.lock ./
RUN cargo build --release

# Copy necessary files to build the actual project
RUN rm -r src/
COPY ./ ./

# Prevent some caching thing idk
RUN touch ./src/bin/fritz-app.rs

# Build the actual project
ENV SQLX_OFFLINE=true
RUN cargo build --release --bin fritz-app

# Run the built binary
FROM debian:bookworm-slim

COPY --from=builder /fritz-app/target/release/fritz-app /usr/local/bin/
CMD ["/usr/local/bin/fritz-app"]

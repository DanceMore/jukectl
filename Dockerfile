# Use the official Rust image as the base image
FROM rust:1.88 as builder
WORKDIR /app
COPY . .
RUN cargo build --release --package jukectl-server

### stage 2
# Create a new lightweight image with just the binary
FROM debian:bookworm-slim

# Set the working directory inside the container
WORKDIR /app

# Copy the binary from the builder stage to the final image
COPY --from=builder /app/target/release/jukectl-server /app/jukectl-server

ENV ROCKET_ADDRESS="0.0.0.0"
ENV ROCKET_PROFILE="production"

# Expose the port your Rocket server will listen on (change to your port)
EXPOSE 8000

# Command to run your Rocket application
CMD ["/app/jukectl-server"]

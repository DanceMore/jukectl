# Use the official Rust image as the base image
FROM rust:latest as builder

# Set the working directory inside the container
WORKDIR /app

# Copy the Cargo.toml and Cargo.lock files to cache dependencies
COPY Cargo.toml Cargo.lock ./

# Create an empty project to cache dependencies
RUN mkdir src && echo 'fn main() {}' > src/main.rs

# Build the application to cache dependencies
RUN cargo build --release

# Copy the entire source code into the container
COPY . .

# Build the release version of the application
RUN cargo build --release

### stage 2
# Create a new lightweight image with just the binary
FROM debian:bookworm-slim

# Set the working directory inside the container
WORKDIR /app

# Copy the binary from the builder stage to the final image
COPY --from=builder /app/target/release/jukectl /app/jukectl

ENV ROCKET_ADDRESS="0.0.0.0"

# Expose the port your Rocket server will listen on (change to your port)
EXPOSE 8000

# Command to run your Rocket application
CMD ["/app/jukectl"]

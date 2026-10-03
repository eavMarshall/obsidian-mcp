# Stage 1: Build
FROM rust:alpine AS builder
# Install musl-dev for static linking on Alpine
RUN apk add --no-cache musl-dev
WORKDIR /app
COPY . .
# Build the highly-optimized release binary
RUN cargo build --release

# Stage 2: Runtime
FROM alpine:latest
WORKDIR /app

# Copy the static binary from the builder stage
COPY --from=builder /app/target/release/obsidian-mcp /usr/local/bin/obsidian-mcp

# Expose standard stdio
# The MCP client will communicate via docker run -i
ENTRYPOINT ["obsidian-mcp"]

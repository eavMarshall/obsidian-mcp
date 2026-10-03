#!/bin/bash
# Check if the image exists. If not, build it.
docker image inspect obsidian-mcp:latest >/dev/null 2>&1 || docker build -t obsidian-mcp:latest .

# Run the container interactively (required for MCP stdin/stdout)
# Mounts the current directory so config.yaml saves to your host machine
docker run -i -v "$(pwd):/app" obsidian-mcp:latest

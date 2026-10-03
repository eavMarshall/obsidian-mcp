#!/bin/sh
# Launch the MCP Server natively without any stdout pollution.
# Mounts the caller's working directory to /workspace
# Mounts the directory containing this script to /app to save config.yaml
DIR="$(cd "$(dirname "$0")" && pwd)"
docker run -i --rm -v "$(pwd):/workspace" -v "${DIR}:/app" obsidian-mcp:latest

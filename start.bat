@echo off
REM Check if the image exists. If not, build it.
docker image inspect obsidian-mcp:latest >nul 2>&1 || docker build -t obsidian-mcp:latest .

REM Run the container interactively (required for MCP stdin/stdout)
REM Mounts the current directory so config.yaml saves to your host machine
docker run -i -v "%cd%:/app" obsidian-mcp:latest

@echo off
REM Launch the MCP Server natively without any stdout pollution.
REM Mounts the caller's working directory to /workspace
REM Mounts the directory containing this script to /app to save config.yaml
docker run -i --rm -v "%cd%:/workspace" -v "%~dp0.:/app" obsidian-mcp:latest

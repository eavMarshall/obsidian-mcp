@echo off
echo Rebuilding Obsidian MCP Gateway from scratch...
docker build --no-cache -t obsidian-mcp:latest .

echo.
echo Rebuild complete! Starting Gateway...
docker run -i -v "%cd%:/app" obsidian-mcp:latest

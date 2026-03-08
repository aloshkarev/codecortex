# Service Templates

The files `com.codecortex.mcp.plist` and `cortex-mcp.service` are templates.
They include placeholders that must be substituted before installation:

- `%BIN_PATH%` absolute path to the `cortex` binary
- `%BIN_DIR%` directory containing `cortex`
- `%HOME%` user home directory
- `%LOG_DIR%` log directory (for launchd plist)
- `%USER%` service user (for systemd unit)

The installer scripts generate concrete service files directly:

- `install.sh`
- `scripts/cortex-service.sh`

If you install manually from templates, replace placeholders first.

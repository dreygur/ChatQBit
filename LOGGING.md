# Logging Configuration

ChatQBit supports flexible logging to both console and file.

## Configuration

Add these options to your `.env` file:

```env
# Enable file logging (in addition to console)
LOG_TO_FILE=true

# Path to log file
LOG_FILE_PATH=logs/chatqbit.log
```

## Options

### `LOG_TO_FILE`
- **Type:** Boolean (`true` or `false`)
- **Default:** `false`
- **Description:** When enabled, logs are written to both console AND file

### `LOG_FILE_PATH`
- **Type:** String (file path)
- **Default:** `chatqbit.log` (in current directory)
- **Description:** Path where log file will be created
- **Examples:**
  - `chatqbit.log` - Current directory
  - `logs/chatqbit.log` - In logs subdirectory
  - `/var/log/chatqbit.log` - Absolute path

## Features

✅ **Dual Output:** Logs to console and file simultaneously
✅ **Auto-create:** Creates log directory if it doesn't exist
✅ **Append Mode:** Appends to existing log file (doesn't overwrite)
✅ **No ANSI:** File logs have no color codes (clean text)
✅ **Timestamped:** All logs include timestamps

## Log Levels

By default, logs at these levels:
- `INFO`: General information
- `DEBUG`: Detailed debugging (chatqbit module)
- `WARN`: Warnings
- `ERROR`: Errors
- `TRACE`: Very detailed (reqwest, tower_http)

### Custom Log Level

Set the `RUST_LOG` environment variable:

```env
RUST_LOG=debug  # More verbose
RUST_LOG=info   # Default
RUST_LOG=warn   # Less verbose
```

Or per-module:
```env
RUST_LOG=info,chatqbit=debug,fileserver=trace
```

## Examples

### Console Only (Default)
```env
LOG_TO_FILE=false
```

Output only goes to terminal.

### File Only via Redirect
```bash
cargo run --release > /dev/null 2> logs/chatqbit.log
```

### Both Console and File (Recommended)
```env
LOG_TO_FILE=true
LOG_FILE_PATH=logs/chatqbit.log
```

Logs appear in terminal AND are saved to file.

### Production Setup
```env
LOG_TO_FILE=true
LOG_FILE_PATH=/var/log/chatqbit/chatqbit.log
RUST_LOG=info
```

## Log Rotation

For production, use `logrotate`:

```bash
# Create /etc/logrotate.d/chatqbit
sudo nano /etc/logrotate.d/chatqbit
```

```
/var/log/chatqbit/*.log {
    daily
    rotate 7
    compress
    delaycompress
    missingok
    notifempty
    create 0644 rakib rakib
}
```

## Viewing Logs

### Tail in real-time
```bash
tail -f logs/chatqbit.log
```

### Last 100 lines
```bash
tail -n 100 logs/chatqbit.log
```

### Search for errors
```bash
grep ERROR logs/chatqbit.log
```

### Search for specific torrent
```bash
grep "abc123" logs/chatqbit.log
```

### Filter by date
```bash
grep "2025-10-17" logs/chatqbit.log
```

## Log Format

Each log entry includes:
```
2025-10-17T12:34:56.789Z INFO chatqbit: Bot started successfully!
│                        │    │         └─ Message
│                        │    └─ Module
│                        └─ Level
└─ Timestamp (UTC)
```

## Troubleshooting

### Permission Denied
If you get permission errors:
```bash
# Make sure directory exists and is writable
mkdir -p logs
chmod 755 logs
```

### Disk Space
Monitor disk usage:
```bash
du -sh logs/
```

### Too Verbose
Reduce log level:
```env
RUST_LOG=warn
```

### Missing Logs
Check settings:
```bash
grep LOG .env
```

## Performance

File logging has minimal performance impact:
- Async writes (non-blocking)
- Buffered I/O
- No impact on bot responsiveness

## Security

⚠️ **Important:** Log files may contain sensitive information:
- Torrent hashes
- File paths
- Telegram message IDs
- qBittorrent settings

**Best practices:**
1. Restrict log file permissions:
   ```bash
   chmod 600 logs/chatqbit.log
   ```

2. Don't commit logs to git (already in `.gitignore`)

3. Rotate and delete old logs regularly

4. Don't share log files publicly

## Integration with systemd

If running as systemd service:

```ini
[Service]
# ... other settings ...
Environment="LOG_TO_FILE=true"
Environment="LOG_FILE_PATH=/var/log/chatqbit/chatqbit.log"
StandardOutput=journal
StandardError=journal
```

View with:
```bash
journalctl -u chatqbit -f
```

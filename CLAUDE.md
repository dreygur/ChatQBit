# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

ChatQBit is a Rust-based Telegram bot for remote control of qBittorrent clients. The project uses a Cargo workspace architecture with four crates following clean architecture principles:

- **chatqbit** (binary): Main entry point, handles initialization and dependency wiring
- **telegram**: Bot logic, command handlers, dialogue state management, interactive menus
- **torrent**: Clean API wrapper around qbit-rs for qBittorrent operations
- **fileserver**: HTTP server for streaming torrent files with range request support

## Essential Commands

```bash
# Development
cargo build                    # Debug build
cargo run                      # Run bot (requires .env)
cargo test                     # Run all tests

# Testing specific crates
cargo test -p torrent          # Test torrent crate only
cargo test -p telegram         # Test telegram crate only
cargo test test_login          # Run specific test

# Production
cargo build --release          # Optimized build (LTO enabled)
cargo run --release           # Run release build
```

## Environment Setup

The `.env` file is required in project root:
```
TELOXIDE_TOKEN=<telegram_bot_token>
QBIT_HOST=http://127.0.0.1:8080
QBIT_USERNAME=admin
QBIT_PASSWORD=<password>

# File Server Configuration (Optional - for streaming)
FILE_SERVER_HOST=0.0.0.0
FILE_SERVER_PORT=8081
FILE_SERVER_BASE_URL=http://localhost:8081
FILE_SERVER_SECRET=change_me_in_production
```

Tests in `crates/torrent/src/torrent.rs` require valid credentials and running qBittorrent instance.

## Architecture & Message Flow

### Initialization (crates/chatqbit/src/main.rs:11-94)

1. Loads `.env` variables via `dotenv()`
2. Initializes tracing with configurable log levels
3. Creates and authenticates `TorrentApi` client
4. Fetches qBittorrent download path for file server
5. Initializes `FileServerApi` with download path, secret, and base URL
6. Spawns file server in background task via `tokio::spawn`
7. Registers bot commands menu via `set_bot_commands()`
8. Builds dispatcher with `InMemStorage<State>` for dialogue management
9. Injects `TorrentApi` and `FileServerApi` as dependencies using `dptree::deps!`

### Message Routing (crates/telegram/src/telegram.rs:17-59)

Uses `dptree` for declarative routing:
- **Command routing**: `teloxide::filter_command` branches on `Command` enum variants
- **State-based routing**: Most commands only work in `State::Start`
- **Dialogue state**: `State::GetMagnet` accepts text messages or document uploads
- **Callback routing**: `Update::filter_callback_query` handles inline keyboard button presses
- **Invalid messages**: Routed to `invalid_state` handler

### State Management

Two dialogue states defined in `crates/telegram/src/types.rs:14-20`:
- `State::Start`: Default state, accepts all bot commands
- `State::GetMagnet`: Awaiting magnet link or .torrent file upload

State transitions via:
- `dialogue.update(State::GetMagnet)` - Enter GetMagnet state
- `dialogue.exit()` - Return to Start state

## Key Implementation Patterns

### DRY Helper Functions

**Hash Command Pattern** (`crates/telegram/src/handlers.rs`):
```rust
execute_hash_command(hash_arg, api, operation, success_msg, usage_msg)
```
Eliminates repetition across resume/pause/delete/recheck/reannounce commands.

**Argument Extraction** (`crates/telegram/src/utils.rs`):
- `extract_hash_arg(msg)` - Validates and extracts hash from command
- `extract_limit_arg(msg)` - Parses speed limit values

**Formatting Utilities** (`crates/telegram/src/handlers.rs`):
- `format_transfer_info()` - Formats global transfer statistics
- `format_torrent_list()` - Formats torrent list with copy-friendly hashes

### Duplicate Torrent Detection

**Location**: `crates/torrent/src/utils.rs` + `crates/telegram/src/constants.rs:11`

**Flow**:
1. Extract info hash from input (magnet URL or .torrent file)
2. Fetch all existing torrents via `client.get_torrent_list()`
3. Build `HashSet<String>` of existing hashes (case-insensitive)
4. Compare input hash against existing hashes
5. Return `DuplicateCheckResult::Duplicate(hash)` if match found

**Magnet Hash Extraction** (`crates/torrent/src/utils.rs:extract_info_hash_from_magnet`):
- Regex: `r"(?i)urn:btih:([a-f0-9]{40})"`
- Supports 40-character hex format only

**Torrent File Hash Extraction** (`crates/torrent/src/utils.rs:extract_info_hash_from_file`):
- Locates bencoded "info" dictionary (`4:infod` pattern)
- Uses depth tracking to find matching closing delimiter
- Computes SHA-1 hash of info dictionary bytes

**Configuration**:
- Toggle via `ENABLE_DUPLICATE_CHECK` constant in `crates/telegram/src/constants.rs:11`
- Fail-open behavior: continues adding if check fails

### Interactive Keyboard Menus

**Keyboard Builders** (`crates/telegram/src/keyboards.rs`):
- `main_menu_keyboard()` - Primary bot menu with common actions
- `torrent_action_keyboard(hash)` - Per-torrent action buttons
- `confirmation_keyboard(action, data)` - Destructive operation confirmations

**Callback Handling** (`crates/telegram/src/callbacks.rs`):
- Callback data format: `action:parameter` (e.g., `delete:abc123`)
- `handle_callback()` parses data and routes to appropriate handler
- Updates original message after action completion

### Torrent File Upload

**Handler**: `crates/telegram/src/commands.rs:magnet()` (State::GetMagnet)

**Validation Sequence**:
1. Check file extension is `.torrent`
2. Verify file not empty
3. Validate bencoded format (starts with 'd')
4. Download file via `bot.get_file()` + `bot.download_file()`
5. Extract info hash and check duplicates
6. Call `api.add_torrent_file(filename, bytes)`

**Error Handling**:
- User-friendly error messages with `emoji::ERROR` prefix
- Comprehensive tracing for debugging
- Transaction-like flow: file validation before API calls

### Command Handler Pattern

Standard handler structure (see `crates/telegram/src/commands.rs`):
```rust
pub async fn command_name(
    bot: Bot,
    msg: Message,
    api: TorrentApi,
) -> HandlerResult {
    // 1. Extract and validate arguments
    let arg = extract_*_arg(&msg)?;

    // 2. Execute operation via TorrentApi
    api.operation(arg).await?;

    // 3. Format response with emoji constants
    let response = format!("{} Success message", emoji::SUCCESS);

    // 4. Send response and log errors
    bot.send_message(msg.chat.id, response).await?;
    Ok(())
}
```

## TorrentApi Interface

**Core Methods** (`crates/torrent/src/torrent.rs`):

```rust
// Initialization
TorrentApi::new() -> Self                           // From env vars
login() -> Result<(), Error>                        // Authenticate

// Torrent operations
query() -> Result<Vec<Torrent>, Error>              // List (limit 10)
magnet(&[String]) -> Result<(), Error>              // Add via URL
add_torrent_file(&str, Vec<u8>) -> Result<(), Error>  // Add via file
check_duplicates(&[String]) -> Result<DuplicateCheckResult, Error>

// Torrent control (all take single hash)
start_torrents(&str) -> Result<(), Error>
stop_torrents(&str) -> Result<(), Error>
delete_torrents(&str, bool) -> Result<(), Error>
recheck_torrents(&str) -> Result<(), Error>
reannounce_torrents(&str) -> Result<(), Error>

// Priority management
set_top_priority(&str) -> Result<(), Error>
set_bottom_priority(&str) -> Result<(), Error>

// Global operations
get_transfer_info() -> Result<TransferInfo, Error>
get_version() -> Result<String, Error>
get_categories() -> Result<HashMap<String, Category>, Error>
get_tags() -> Result<Vec<String>, Error>

// Speed limits
get_download_limit() -> Result<u64, Error>
get_upload_limit() -> Result<u64, Error>
set_download_limit(u64) -> Result<(), Error>
set_upload_limit(u64) -> Result<(), Error>

// Streaming support (NEW)
get_torrent_files(&str) -> Result<Vec<TorrentContent>, Error>
set_file_priority(&str, Vec<i64>, Priority) -> Result<(), Error>
toggle_sequential_download(&str) -> Result<(), Error>
toggle_first_last_piece_priority(&str) -> Result<(), Error>
get_default_save_path() -> Result<PathBuf, Error>
```

**Thread Safety**: `TorrentApi` wraps `Qbit` client in `Arc` for safe cloning across handlers.

## Streaming Architecture

### Overview

The streaming feature allows users to play torrent files directly in video players (VLC, MX Player, browsers) without waiting for complete download. This is achieved through:

1. **Sequential Download Mode**: Downloads pieces in order (better for streaming)
2. **HTTP File Server**: Serves files with range request support for seeking
3. **Secure Token System**: SHA-256 based tokens for access control

### Components

**File Server** (`crates/fileserver/`):
- **server.rs**: Axum-based HTTP server with range request handling
- **state.rs**: Thread-safe stream registry with `Arc<RwLock<HashMap>>`
- **token.rs**: SHA-256 token generation and verification

**Architecture Flow**:
```
User -> /stream <hash> -> Bot
                          ├─> TorrentApi.get_torrent_files(hash)
                          ├─> Generate tokens for each file
                          ├─> Register streams in FileServerApi.state
                          └─> Return streaming URLs

User clicks URL -> FileServer
                   ├─> Verify token
                   ├─> Locate file in download directory
                   ├─> Handle HTTP range requests
                   └─> Stream file chunks
```

### Streaming Commands

**`/stream <hash>`** (`crates/telegram/src/commands.rs:stream`):
1. Fetches torrent files via `get_torrent_files(hash)`
2. Gets torrent save path from `get_torrent_info(hash).save_path`
3. Generates secure token per file: `SHA256(hash + file_index + secret)`
4. Registers stream in server state with file path and metadata
5. Returns clickable URLs: `{BASE_URL}/stream/{token}/{filename}`
6. Skips files smaller than 1MB (likely samples/metadata)

**`/files <hash>`** (`crates/telegram/src/commands.rs:files`):
- Lists all files in torrent with size and download progress
- Useful for multi-file torrents to identify which files to stream

**`/sequential <hash>`** (`crates/telegram/src/commands.rs:sequential`):
- Toggles sequential download mode for streaming optimization
- Also enables first/last piece priority for faster header loading
- Pieces downloaded in order instead of rarest-first

### HTTP Server Details

**URL Format**: `http://{HOST}:{PORT}/stream/{token}/{filename}`

**Range Request Support** (`crates/fileserver/src/server.rs:handle_range_request`):
- Parses `Range: bytes=start-end` header
- Seeks to requested position in file
- Returns `206 Partial Content` with proper headers
- Enables video seeking and progressive playback

**Security**:
- Token verification before serving files
- Tokens tied to specific torrent hash + file index
- Configurable secret key via `FILE_SERVER_SECRET`
- Stream registration prevents unauthorized access

**MIME Type Detection**:
- Automatic content-type detection via `mime_guess`
- Proper headers for browser compatibility

### State Management

**ServerState** (`crates/fileserver/src/state.rs`):
```rust
struct ServerState {
    streams: Arc<RwLock<HashMap<String, StreamInfo>>>,
    download_path: PathBuf,
    secret: String,
}

struct StreamInfo {
    torrent_hash: String,
    file_index: usize,
    file_path: PathBuf,
    filename: String,
    created_at: DateTime<Utc>,
}
```

**Stream Lifecycle**:
1. Registration: `/stream` command adds entry to HashMap
2. Access: File server looks up token in HashMap
3. Cleanup: `cleanup_old_streams(max_age_hours)` removes expired entries

### Integration Points

**Dependency Injection** (`crates/chatqbit/src/main.rs:89`):
```rust
Dispatcher::builder(bot, telegram::schema())
    .dependencies(dptree::deps![InMemStorage::<State>::new(), client, file_server])
```

**Background Server** (`crates/chatqbit/src/main.rs:75`):
```rust
tokio::spawn(async move {
    file_server_clone.serve(&host, port).await
});
```

The file server runs concurrently with the Telegram bot, both sharing the same tokio runtime.

### Production Considerations

**Security**:
- Change `FILE_SERVER_SECRET` from default value
- Use HTTPS reverse proxy (nginx/caddy) for public deployment
- Consider IP whitelisting or additional authentication

**Performance**:
- File server uses tokio for async I/O
- Range requests allow efficient seeking without loading entire files
- CORS enabled for browser-based players

**Networking**:
- `FILE_SERVER_BASE_URL` must be accessible from client devices
- For remote access, configure port forwarding or use tunneling (ngrok, cloudflared)
- Default localhost:8081 only works on same machine

## Important Constants

**Display Configuration** (`crates/telegram/src/constants.rs`):
- `HASH_DISPLAY_LENGTH`: 8 chars (unused - full hashes shown for copy-ability)
- `MAX_TORRENTS_DISPLAY`: 50 torrents max in list
- `ENABLE_DUPLICATE_CHECK`: Toggle duplicate detection

**Emoji Prefix Standards**:
- `SUCCESS` (✅): Successful operations
- `ERROR` (❌): Errors and failures
- `INFO` (📊): Information displays
- `DOWNLOAD` (📥) / `UPLOAD` (📤): Transfer stats

## Testing Requirements

Tests require:
1. `.env` file with valid credentials
2. Running qBittorrent instance at `QBIT_HOST`
3. Network access to qBittorrent Web UI

Test organization:
- `test_login`: Verifies authentication flow
- `test_query`: Validates torrent list fetching
- Uses `dotenv().ok()` for environment loading

## Code Style & Type Safety

- **Error handling**: Custom `BotError` enum + `HandlerResult` type alias
- **Lifetimes**: Explicit annotations in utility functions
- **Logging**: `tracing` crate with structured logging throughout
- **Result types**: `BotResult<T>` and `HandlerResult` aliases for consistency
- **Strong typing**: Command enum with `BotCommands` derive macro

## Release Build Configuration

Optimizations in `crates/chatqbit/Cargo.toml:18-23`:
- `opt-level = 3`: Maximum optimization
- `lto = true`: Link-time optimization across crates
- `codegen-units = 1`: Better optimization at cost of compile time
- `strip = true`: Remove debug symbols for smaller binary

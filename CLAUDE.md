# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

ChatQBit is a feature-rich Telegram bot written in Rust that provides comprehensive remote control of a qBittorrent client via Telegram. The project is organized as a Cargo workspace with three crates following clean architecture principles:

- **chatqbit** (main binary): Entry point that initializes the bot, authenticates with qBittorrent, and wires up dependencies
- **telegram**: Handles Telegram bot logic, commands, dialogue state management, message routing, and user interaction formatting
- **torrent**: Provides a clean API wrapper around qbit-rs for qBittorrent operations

## Required Environment Variables

Create a `.env` file in the project root with:

```
TELOXIDE_TOKEN=<Your Telegram Bot Token>
QBIT_HOST=<QBitTorrent Web UI address with PORT. Default: http://127.0.0.1:8080>
QBIT_USERNAME=<QBitTorrent Username. Default: admin>
QBIT_PASSWORD=<QBitTorrent Password>
```

## Build and Run Commands

```bash
# Build the project
cargo build

# Build optimized release version
cargo build --release

# Run the bot (development)
cargo run

# Run the bot (release)
cargo run --release

# Run tests
cargo test

# Run tests for a specific crate
cargo test -p torrent
cargo test -p telegram
cargo test -p chatqbit

# Run a specific test
cargo test test_login
```

## Architecture

### Module Organization

**Telegram Crate Structure:**
```
telegram/
├── commands.rs      # Command handler implementations
├── constants.rs     # Constants, emoji, usage messages
├── error.rs         # Custom error types and result aliases
├── handlers.rs      # Reusable command handler patterns
├── lib.rs          # Module exports
├── telegram.rs     # Message routing and dispatcher setup
├── types.rs        # Bot commands and state definitions
└── utils.rs        # Formatting and parsing utilities
```

### Application Flow

1. **Initialization** (crates/chatqbit/src/main.rs:11-46):
   - Loads environment variables from `.env`
   - Initializes structured logging with tracing
   - Creates and authenticates TorrentApi client
   - Sets up teloxide dispatcher with InMemStorage for dialogue state
   - Injects TorrentApi as dependency for handlers

2. **Message Routing** (crates/telegram/src/telegram.rs:10-44):
   - Uses dptree for declarative message routing
   - Routes based on Command enum and dialogue State
   - All routes properly wired to handler functions
   - Invalid messages routed to `invalid_state` handler

3. **State Management**:
   - `State::Start`: Default state, accepts all commands
   - `State::GetMagnet`: Waiting for user to send magnet/torrent URL
   - State transitions managed via `dialogue.update(State)` and `dialogue.exit()`

### Key Design Patterns

**Error Handling:**
- Custom `BotError` enum for type-safe error handling
- Consistent error formatting with emoji prefixes
- Comprehensive tracing for debugging

**DRY Principles:**
- `execute_hash_command()` helper eliminates repetitive hash command patterns
- `format_*()` functions in handlers.rs provide consistent output formatting
- Constants module centralizes all string literals and emojis
- Utils module provides reusable parsing and formatting functions

**Command Handler Pattern:**
All handlers follow this structure:
1. Parse and validate arguments using `utils::extract_*_arg()`
2. Execute operation via TorrentApi
3. Format response with emoji constants
4. Send response and log errors

**Type Safety:**
- Strong typing for commands, states, and errors
- Result type aliases (`HandlerResult`, `BotResult`)
- Proper lifetime annotations in utility functions

### Available Commands

**Torrent Management:**
- `/list` - List all torrents with status and progress
- `/info <hash>` - Detailed torrent information
- `/start <hash|all>` - Start/resume torrents
- `/stop <hash|all>` - Stop/pause torrents
- `/delete <hash>` - Delete torrent (keep files)
- `/deletedata <hash>` - Delete torrent with files
- `/recheck <hash>` - Recheck torrent data
- `/reannounce <hash>` - Reannounce to trackers

**Priority Control:**
- `/topprio <hash>` - Set maximum priority
- `/bottomprio <hash>` - Set minimum priority

**System Information:**
- `/transferinfo` - Global transfer statistics
- `/version` - qBittorrent version
- `/categories` - List all categories
- `/tags` - List all tags
- `/speedlimits` - Show current speed limits

**Speed Control:**
- `/setdllimit <bytes/s>` - Set download limit (0 = unlimited)
- `/setupllimit <bytes/s>` - Set upload limit (0 = unlimited)

### TorrentApi Interface

**Core Methods** (crates/torrent/src/torrent.rs):
- `new()` - Create client from environment variables
- `login()` - Authenticate with qBittorrent
- `query()` - Fetch torrent list (limit 10)
- `magnet(&[String])` - Add torrents by URL
- `get_torrent_info(&str)` - Get detailed torrent properties
- `start_torrents/stop_torrents/delete_torrents` - Torrent control
- `set_*_priority` - Priority management
- `get/set_*_limit` - Speed limit management
- `get_categories/get_tags` - Metadata retrieval

## Testing Notes

- Tests in crates/torrent/src/torrent.rs require `.env` file with valid qBittorrent credentials
- `test_login`: Verifies authentication with qBittorrent server
- `test_query`: Tests fetching torrent list after authentication
- Tests use `dotenv().ok()` to load environment variables

## Release Profile

The release build (crates/chatqbit/Cargo.toml:18-23) is optimized for size and performance:
- `opt-level = 3`: Maximum optimization
- `lto = true`: Link-time optimization
- `codegen-units = 1`: Single codegen unit for better optimization
- `strip = true`: Strip symbols from binary

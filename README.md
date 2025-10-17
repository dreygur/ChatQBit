# ChatQBit Documentation

Project documentation and architecture reference.

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
├── callbacks.rs     # Inline keyboard callback handlers
├── commands.rs      # Command handler implementations
├── constants.rs     # Constants, emoji, usage messages
├── error.rs         # Custom error types and result aliases
├── handlers.rs      # Reusable command handler patterns
├── keyboards.rs     # Inline keyboard builders
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

**Duplicate Prevention:**
- Automatic duplicate detection when adding torrents via magnet links or .torrent files
- Extracts info hash from magnet URLs using regex parsing
- Extracts info hash from .torrent files by parsing bencoded data and hashing the info dictionary
- Case-insensitive hash matching for reliability
- Configurable via `ENABLE_DUPLICATE_CHECK` constant in constants.rs
- Fail-open behavior: continues adding if duplicate check fails
- User-friendly warning message showing duplicate hash

**User Experience Improvements:**
- Full torrent hashes displayed in `/list` command (not truncated)
- Hashes formatted in monospace (backticks) for easy tap-to-copy in Telegram
- Helpful tips in usage messages directing users to `/list` for hashes
- Visual tip at bottom of list reminding users to tap monospace hash to copy

**Interactive Bot Menus:**
- Telegram bot command menu registered automatically on startup
- Main interactive menu via `/menu` command with inline keyboard buttons
- Per-torrent action keyboards (start, stop, info, delete, priority)
- Confirmation dialogs for destructive operations (delete with data)
- Speed limit configuration keyboard with quick actions
- Callback query handling for all button interactions
- Pagination support for long torrent lists (ready for implementation)

**Torrent File Upload:**
- Supports adding torrents via .torrent file uploads in addition to magnet links
- Uses `/magnet` command to enter GetMagnet state, then accepts either text or document
- Comprehensive file validation:
  - Checks file extension is `.torrent`
  - Validates file is not empty
  - Verifies file format (must start with 'd' for bencoded dictionary)
- Downloads file from Telegram servers using `bot.get_file()` and `bot.download_file()`
- Extracts info hash from .torrent file by:
  - Locating the "info" dictionary in bencoded data (searches for "4:infod" pattern)
  - Finding matching end delimiter using depth tracking
  - Computing SHA-1 hash of the info dictionary bytes
- Duplicate checking works for both magnet links and .torrent files
- User-friendly error messages for invalid files
- Production-ready with comprehensive error handling and logging

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
- `/start` - Welcome message and main menu
- `/menu` - Interactive menu with buttons
- `/list` - List all torrents with status and progress
- `/magnet` - Add torrent via magnet link, URL, or .torrent file upload
- `/info <hash>` - Detailed torrent information
- `/resume <hash|all>` - Resume/start torrents
- `/pause <hash|all>` - Pause/stop torrents
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
- `magnet(&[String])` - Add torrents by URL (magnet links or HTTP)
- `add_torrent_file(&str, Vec<u8>)` - Add torrent from .torrent file data
- `check_duplicates(&[String])` - Check if torrents already exist
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
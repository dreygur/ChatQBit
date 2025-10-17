use dotenv::dotenv;
use telegram::{telegram, State};
use teloxide::{
    prelude::*,
    dispatching::dialogue::InMemStorage,
};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, fmt, EnvFilter};
use std::path::PathBuf;

#[tokio::main]
async fn main() {
    // Load environment variables from .env file
    dotenv().ok();

    // Initialize tracing with optional file logging
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "info,chatqbit=debug,reqwest=trace,tower_http=trace".into());

    // Check if file logging is enabled
    let enable_file_logging = std::env::var("LOG_TO_FILE")
        .unwrap_or_else(|_| "false".to_string())
        .parse::<bool>()
        .unwrap_or(false);

    let log_file_path = std::env::var("LOG_FILE_PATH")
        .unwrap_or_else(|_| "chatqbit.log".to_string());

    if enable_file_logging {
        // Create logs directory if it doesn't exist
        if let Some(parent) = std::path::Path::new(&log_file_path).parent() {
            std::fs::create_dir_all(parent).ok();
        }

        // Open log file
        let log_file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file_path)
            .expect("Failed to open log file");

        // Initialize with both console and file output
        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt::layer()) // Console output
            .with(fmt::layer().with_writer(std::sync::Arc::new(log_file)).with_ansi(false)) // File output (no colors)
            .init();

        eprintln!("📝 Logging to file: {}", log_file_path);
    } else {
        // Console-only logging
        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt::layer())
            .init();
    }

    // Initialize the bot from environment variables
    let bot = Bot::from_env();

    // Initialize qBittorrent API client
    let client = torrent::TorrentApi::new();

    // Authenticate with qBittorrent
    if let Err(e) = client.login().await {
        eprintln!("Failed to login to qBittorrent: {}", e);
        eprintln!("Please check your credentials in the .env file");
        return;
    }

    info!("Bot started successfully!");
    info!("qBittorrent client authenticated");

    // Get qBittorrent download path
    let download_path = match client.get_default_save_path().await {
        Ok(path) => {
            info!("qBittorrent download path: {}", path.display());
            path
        }
        Err(e) => {
            tracing::warn!("Failed to get qBittorrent download path, using default: {}", e);
            PathBuf::from(std::env::var("QBIT_DOWNLOAD_PATH").unwrap_or_else(|_| "/downloads".to_string()))
        }
    };

    // Initialize file server
    let file_server_host = std::env::var("FILE_SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let file_server_port = std::env::var("FILE_SERVER_PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(8081);
    let mut file_server_base_url = std::env::var("FILE_SERVER_BASE_URL")
        .unwrap_or_else(|_| format!("http://localhost:{}", file_server_port));
    let file_server_secret = std::env::var("FILE_SERVER_SECRET")
        .unwrap_or_else(|_| "change_me_in_production".to_string());

    info!("File server will listen on {}:{}", file_server_host, file_server_port);

    // Check if tunnel is enabled
    let tunnel_provider = std::env::var("TUNNEL_PROVIDER")
        .unwrap_or_else(|_| "none".to_string())
        .parse::<fileserver::TunnelProvider>()
        .unwrap_or(fileserver::TunnelProvider::None);

    // Start tunnel if configured
    if tunnel_provider != fileserver::TunnelProvider::None {
        info!("🚇 Starting tunnel with provider: {:?}", tunnel_provider);
        match fileserver::start_tunnel(tunnel_provider, file_server_port).await {
            Ok(tunnel_info) => {
                info!("✅ Tunnel established successfully!");
                info!("🌐 Public URL: {}", tunnel_info.public_url);
                info!("📡 Provider: {}", tunnel_info.provider);

                // Use tunnel URL as base URL
                file_server_base_url = tunnel_info.public_url;
            }
            Err(e) => {
                tracing::warn!("⚠️  Failed to start tunnel: {}", e);
                tracing::warn!("Continuing with local URL: {}", file_server_base_url);
            }
        }
    } else {
        info!("No tunnel configured, using local URL: {}", file_server_base_url);
    }

    let file_server = fileserver::FileServerApi::new(
        download_path,
        file_server_secret,
        file_server_base_url,
        client.clone(),
    );

    // Spawn file server in background
    let file_server_clone = file_server.clone();
    let file_server_host_clone = file_server_host.clone();
    tokio::spawn(async move {
        if let Err(e) = file_server_clone.serve(&file_server_host_clone, file_server_port).await {
            tracing::error!("File server error: {}", e);
        }
    });

    // Set bot commands menu
    if let Err(e) = telegram::set_bot_commands(&bot).await {
        tracing::warn!("Failed to set bot commands menu: {}", e);
    } else {
        info!("Bot commands menu registered");
    }

    Dispatcher::builder(bot, telegram::schema())
        .dependencies(dptree::deps![InMemStorage::<State>::new(), client, file_server])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}

use dotenv::dotenv;
use telegram::{telegram, State};
use teloxide::{
    prelude::*,
    dispatching::dialogue::InMemStorage,
};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    // Load environment variables from .env file
    dotenv().ok();

    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,chatqbit=debug,reqwest=trace,tower_http=trace".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

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

    // Set bot commands menu
    if let Err(e) = telegram::set_bot_commands(&bot).await {
        tracing::warn!("Failed to set bot commands menu: {}", e);
    } else {
        info!("Bot commands menu registered");
    }

    Dispatcher::builder(bot, telegram::schema())
        .dependencies(dptree::deps![InMemStorage::<State>::new(), client])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}

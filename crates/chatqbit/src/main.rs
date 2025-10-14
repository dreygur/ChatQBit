use telegram::{telegram, State};
use teloxide::dispatching::dialogue::InMemStorage;
use teloxide::prelude::*;

#[tokio::main]
async fn main() {
    // Load environment variables from .env file
    let _ = dotenv::dotenv();

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

    println!("Bot started successfully!");
    println!("qBittorrent client authenticated");

    Dispatcher::builder(bot, telegram::schema())
        .dependencies(dptree::deps![InMemStorage::<State>::new(), client])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}

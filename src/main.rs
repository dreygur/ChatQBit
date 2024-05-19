use teloxide::{
  dispatching::{dialogue, dialogue::InMemStorage, UpdateHandler},
  prelude::*,
  utils::command::BotCommands,
};
use torrent::TorrentApi;

type MyDialogue = Dialogue<State, InMemStorage<State>>;
type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

mod torrent;

#[derive(Clone, Default)]
pub enum State {
  #[default]
  Start,
  GetMagnet,
}

#[derive(BotCommands, Clone)]
#[command(
  rename_rule = "lowercase",
  description = "These commands are supported:"
)]
enum Command {
  #[command(description = "display this text.")]
  Help,
  #[command(description = "start the torrent download")]
  // Start,
  // #[command(description = "start the torrent download")]
  Magnet,
  #[command(description = "cancel the purchase procedure.")]
  Cancel,
}

#[tokio::main]
async fn main() {
  let bot = Bot::from_env();

  // initialize client with given username and password
  let client = torrent::TorrentApi::new();

  // login first
  let _ = client.login().await;

  println!("The bot is now started...");

  Dispatcher::builder(bot, schema())
    .dependencies(dptree::deps![InMemStorage::<State>::new(), client])
    .enable_ctrlc_handler()
    .build()
    .dispatch()
    .await;
}

fn schema() -> UpdateHandler<Box<dyn std::error::Error + Send + Sync + 'static>> {
  use dptree::case;

  let command_handler = teloxide::filter_command::<Command, _>()
    .branch(
      case![State::Start]
        .branch(case![Command::Help].endpoint(help))
        // .branch(case![Command::Start].endpoint(start))
        .branch(case![Command::Magnet].endpoint(get_magnet)),
    )
    .branch(case![Command::Cancel].endpoint(cancel));

  let message_handler = Update::filter_message()
    .branch(command_handler)
    .branch(case![State::GetMagnet].endpoint(magnet))
    .branch(dptree::endpoint(invalid_state));

  dialogue::enter::<Update, InMemStorage<State>, State, _>().branch(message_handler)
}

// async fn start(bot: Bot, msg: Message) -> HandlerResult {
//   bot.send_message(msg.chat.id, "Let's start!").await?;
//   Ok(())
// }

async fn help(bot: Bot, msg: Message) -> HandlerResult {
  bot
    .send_message(msg.chat.id, Command::descriptions().to_string())
    .await?;
  Ok(())
}

async fn cancel(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
  bot
    .send_message(msg.chat.id, "Cancelling the dialogue.")
    .await?;
  dialogue.exit().await?;
  Ok(())
}

async fn get_magnet(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
  bot
    .send_message(msg.chat.id, "Send me the magnet link")
    .await?;
  dialogue.update(State::GetMagnet).await?;
  Ok(())
}

async fn magnet(bot: Bot, msg: Message, torrent: TorrentApi) -> HandlerResult {
  match msg.text().map(ToOwned::to_owned) {
    Some(text) => {
      let urls: [String; 1] = [text];
      match torrent.client.torrents_add_by_url(&urls).await {
        Ok(_) => {
          bot
            .send_message(msg.chat.id, "Torrent has been added to download queue")
            .await?;
        }
        Err(err) => {
          bot.send_message(msg.chat.id, err.to_string()).await?;
        }
      }
    }
    None => {
      bot
        .send_message(msg.chat.id, "Please, send me your magnet link.")
        .await?;
    }
  }
  Ok(())
}

async fn invalid_state(bot: Bot, msg: Message) -> HandlerResult {
  bot
    .send_message(
      msg.chat.id,
      "Unable to handle the message. Type /help to see the usage.",
    )
    .await?;
  Ok(())
}

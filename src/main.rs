#[macro_use]
extern crate lazy_static;

use std::{
    env::{self, VarError},
    process::exit,
};
use teloxide::{dispatching::dialogue::InMemStorage, prelude::Dispatcher, Bot};

mod domain;
mod tg;

#[tokio::main]
async fn main() {
    pretty_env_logger::init_timed();

    let token = get_token_from_env();

    log::info!("Bot token is \"{token}\"");

    let bot = Bot::new(token);
    let mut dispatcher = Dispatcher::builder(bot, tg::handler::schema())
        .dependencies(teloxide::dptree::deps![
            InMemStorage::<tg::state::DialogueState>::new(),
            domain::fetch::HtmlMenuFetcher::new()
        ])
        .enable_ctrlc_handler()
        .build();

    log::info!("Starting bot...");

    dispatcher.dispatch().await;
}

fn get_token_from_env() -> String {
    env::var("BOT_TOKEN")
        .or_else(|ref e| {
            match e {
                VarError::NotUnicode(_) => {
                    log::error!("BOT_TOKEN was found but does not contain valid unicode - {e}");
                    exit(2);
                }
                VarError::NotPresent => {}
            };

            env::var("TELOXIDE_TOKEN")
        })
        .unwrap_or_else(|ref e| match e {
            VarError::NotUnicode(_) => {
                log::warn!("TELOXIDE_TOKEN was found but does not contain valid unicode - {e}");
                exit(2);
            }
            VarError::NotPresent => {
                log::error!(
                    "No Bot token found. Please provide the token either via the BOT_TOKEN or TELOXIDE_TOKEN environment variable."
                );
                exit(1);
            }
        })
}

pub mod handler {

    use state::DialogueState;

    use teloxide::{
        dispatching::{
            dialogue::{self, InMemStorage},
            UpdateFilterExt, UpdateHandler,
        },
        dptree,
        prelude::Dialogue,
        types::{Me, Message, MessageId, Update},
    };

    use crate::{
        domain::fetch::err::FetcherError,
        domain::model::Canteen,
        tg::command::{Command, DailyArgs},
    };

    type BotDialogue = Dialogue<state::DialogueState, InMemStorage<DialogueState>>;

    type HandlerResult = anyhow::Result<()>;

    pub fn schema() -> UpdateHandler<anyhow::Error> {
        let handle_daily_command = dptree::map(handler::proj::dow_to_naive_date)
            .map_async(handler::proj::fetch_daily_menu)
            .branch(
                dptree::filter_map(
                    |res: Result<crate::domain::model::Menu, std::sync::Arc<anyhow::Error>>| {
                        res.ok()
                    },
                )
                .endpoint(handler::endpoint::menu_by_date),
            )
            .branch(
                dptree::filter_map(
                    |res: Result<crate::domain::model::Menu, std::sync::Arc<anyhow::Error>>| {
                        res.err()
                    },
                )
                .branch(
                    dptree::filter_map(|err: std::sync::Arc<anyhow::Error>| {
                        err.downcast_ref::<FetcherError>().map(|err| err.clone())
                    })
                    .branch(
                        dptree::case![FetcherError::CanteenClosed { canteen, date }]
                            .endpoint(handler::endpoint::err_canteen_closed),
                    ),
                )
                .chain(dptree::inspect(|err: std::sync::Arc<anyhow::Error>| {
                    log::error!("{err}");
                })),
            )
            .branch(dptree::endpoint(handler::endpoint::generic_failure));

        let command_handler = dptree::filter_map(move |message: Message, me: Me| {
            let bot_name = me.user.username.expect("Bots must have a username");
            message
                .text()
                .and_then(|text| Command::parse(text, &bot_name).ok())
        })
        .branch(
            dptree::case![Command::Daily(args)]
                .map(|msg: Message| msg.id)
                .branch(
                    dptree::filter_map(handler::proj::daily_verify_args)
                        .chain(handle_daily_command.clone()),
                )
                .endpoint(handler::endpoint::ask_canteen),
        )
        .branch(dptree::case![Command::Cancel].endpoint(handler::endpoint::cancel));

        let message_handler = Update::filter_message()
            .branch(command_handler)
            .branch(
                dptree::case![DialogueState::Daily { message_id, args }]
                    .map(|(_, args): (MessageId, DailyArgs)| args)
                    .map(|(msg_id, _): (MessageId, DailyArgs)| msg_id)
                    .branch(
                        dptree::filter_map(handler::proj::parse_canteen_from_msg)
                            .map(|args: DailyArgs, canteen: Canteen| {
                                (args.day_of_week, args.canteen.unwrap_or(canteen))
                            })
                            .chain(handle_daily_command.clone()),
                    ),
            )
            .branch(dptree::endpoint(noop_handler));

        dialogue::enter::<Update, InMemStorage<state::DialogueState>, state::DialogueState, _>()
            .branch(message_handler)
    }

    pub mod handler {
        pub mod proj {
            use chrono::NaiveDate;

            use teloxide::prelude::*;

            use crate::{
                domain::fetch::HtmlMenuFetcherWithCache,
                domain::model::{Canteen, DayOfWeek, Menu},
                tg::command::DailyArgs,
            };

            pub fn daily_verify_args(args: DailyArgs) -> Option<(DayOfWeek, Canteen)> {
                let DailyArgs {
                    day_of_week,
                    canteen,
                } = args;

                canteen.map(|canteen| (day_of_week, canteen))
            }

            pub fn dow_to_naive_date((dow, canteen): (DayOfWeek, Canteen)) -> (NaiveDate, Canteen) {
                (dow.into(), canteen)
            }

            pub async fn fetch_daily_menu(
                args: (NaiveDate, Canteen),
                fetcher: HtmlMenuFetcherWithCache,
            ) -> Result<Menu, std::sync::Arc<anyhow::Error>> {
                let (date, canteen) = args;

                let res = fetcher
                    .fetch_daily_menu(date, canteen)
                    .await
                    .map_err(|e| std::sync::Arc::new(e));

                res
            }

            pub fn parse_canteen_from_msg(msg: Message) -> Option<Canteen> {
                let text = msg.text()?.trim();

                Canteen::parser()
                    .parse(text)
                    .ok()
                    .map(|(_, canteen)| canteen)
            }
        }

        pub mod endpoint {
            use chrono::{Datelike, NaiveDate};
            use strum::IntoEnumIterator;
            use teloxide::{
                prelude::*,
                types::{
                    KeyboardButton, KeyboardMarkup, KeyboardRemove, MessageId, ParseMode,
                    ReplyMarkup,
                },
            };

            use crate::{
                domain::model::{Canteen, Menu},
                tg::{
                    command::DailyArgs,
                    handler::{BotDialogue, HandlerResult},
                    state::DialogueState,
                },
            };

            pub async fn cancel(
                bot: Bot,
                message: Message,
                dialogue: BotDialogue,
            ) -> HandlerResult {
                if let Some(state) = dialogue.get_or_default().await.ok() {
                    match state {
                        DialogueState::Noop => {}
                        DialogueState::Daily {
                            message_id,
                            args: _,
                        } => {
                            bot.send_message(message.chat.id, "Befehl abgebrochen 🤖")
                                .reply_to_message_id(message_id)
                                .reply_markup(ReplyMarkup::KeyboardRemove(
                                    KeyboardRemove::new().selective(true),
                                ))
                                .await?;
                        }
                    }

                    dialogue.exit().await?;
                }

                Ok(())
            }

            pub async fn err_canteen_closed(
                bot: Bot,
                msg: Message,
                reply_id: MessageId,
                dialogue: BotDialogue,
                (date, _): (NaiveDate, Canteen),
            ) -> HandlerResult {
                let date_text = if date.weekday().num_days_from_monday() >= 5 {
                    date.format_localized("%A", chrono::Locale::de_DE)
                        .to_string()
                        + "s"
                } else {
                    format!(
                        "am {}",
                        date.format_localized("%A, %d.%m.%Y", chrono::Locale::de_DE)
                    )
                };
                let reply = format!("Die Mensa ist {} leider geschlossen. ☹", date_text);
                dialogue.reset().await?;

                bot.send_message(msg.chat.id, reply)
                    .reply_to_message_id(reply_id)
                    .reply_markup(ReplyMarkup::KeyboardRemove(
                        KeyboardRemove::new().selective(true),
                    ))
                    .await?;

                Ok(())
            }

            /// Sends a generic message about a failed command to the user and resets the dialogue state.
            pub async fn generic_failure(
                bot: Bot,
                msg: Message,
                reply_id: MessageId,
                dialogue: BotDialogue,
            ) -> HandlerResult {
                let reply = "Whoops. Something went wrong.";

                dialogue.reset().await?;

                bot.send_message(msg.chat.id, reply)
                    .reply_to_message_id(reply_id)
                    .reply_markup(ReplyMarkup::KeyboardRemove(
                        KeyboardRemove::new().selective(true),
                    ))
                    .await?;

                Ok(())
            }

            pub async fn menu_by_date(
                bot: Bot,
                msg: Message,
                dialogue: BotDialogue,
                reply_id: MessageId,
                (date, canteen): (NaiveDate, Canteen),
                menu: Menu,
            ) -> HandlerResult {
                let date_fmt = date.format_localized("%A, %d.%m.%Y", chrono::Locale::de_DE);
                let reply = format!(
                    "<strong>Plan für Mensa {} – {}</strong>\n\n",
                    canteen, date_fmt
                ) + &menu.fmt_html()?;

                bot.send_message(msg.chat.id, reply)
                    .parse_mode(ParseMode::Html)
                    .reply_to_message_id(reply_id)
                    .reply_markup(ReplyMarkup::KeyboardRemove(
                        KeyboardRemove::new().selective(true),
                    ))
                    .await?;

                dialogue.reset().await?;

                Ok(())
            }

            pub async fn ask_canteen(
                bot: Bot,
                msg: Message,
                dialogue: BotDialogue,
                reply_id: MessageId,
                args: DailyArgs,
            ) -> HandlerResult {
                dialogue
                    .update(DialogueState::Daily {
                        message_id: reply_id,
                        args: args,
                    })
                    .await?;

                let canteen_btns =
                    Canteen::iter().map(|name| [KeyboardButton::new(format!("Mensa {}", name))]);

                bot.send_message(msg.chat.id, "Bitte Mensa auswählen.")
                    .reply_to_message_id(reply_id)
                    .reply_markup(ReplyMarkup::Keyboard(
                        KeyboardMarkup::new(canteen_btns)
                            .one_time_keyboard(Some(true))
                            .selective(Some(true))
                            .input_field_placeholder(format!("Mensa auswählen")),
                    ))
                    .await?;

                Ok(())
            }
        }
    }

    /// This handler does nothing. It's purpose is to suppress WARN logs for unhandled messages.
    async fn noop_handler() -> HandlerResult {
        Ok(())
    }

    pub mod state {
        use teloxide::types::MessageId;

        use crate::tg::command::DailyArgs;

        #[derive(Clone, Debug, Default)]
        pub enum DialogueState {
            #[default]
            Noop,
            Daily {
                message_id: MessageId,
                args: DailyArgs,
            },
        }
    }
}

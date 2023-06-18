use anyhow::anyhow;
use nom::{
    branch::alt,
    bytes::complete::tag_no_case,
    character::complete::char,
    combinator::{opt, peek},
    IResult,
};
use teloxide::utils::command::ParseError;

use crate::model::{Canteen, DayOfWeek};
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    Cancel,
    Daily(DailyArgs),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DailyArgs {
    pub(super) day_of_week: DayOfWeek,
    pub(super) canteen: Option<Canteen>,
}

impl Command {
    pub fn parse(input: &str, bot_username: &str) -> Result<Self, ParseError> {
        let mut words = input.splitn(2, ' ');

        // unwrap: split iterators always have at least one item
        let mut command_with_botname = words.next().unwrap().split('@');
        let command_text = command_with_botname.next().unwrap();

        let bot_name = command_with_botname.next();
        match bot_name {
            None => {}
            Some(username) => {
                if !username.eq_ignore_ascii_case(bot_username) {
                    return Err(ParseError::WrongBotName(username.to_string()));
                }
            }
        };

        let args_text = words.next().unwrap_or("");

        let (input, _) = char('/')(input).map_err(|_: nom::Err<nom::error::Error<&str>>| {
            ParseError::IncorrectFormat(anyhow!("Commands must begin with '/'").into())
        })?;

        let (command_text, command) = alt((peek(parse_cancel), parse_daily))(input)
            .map_err(|_e| ParseError::UnknownCommand(command_text.to_string()))?;

        match command {
            internal::Command::Cancel => Ok(Command::Cancel),
            internal::Command::Daily => {
                // TODO: Refactor into function

                // Unwrap: This was successfully parsed before by parse_daily
                let (_, day_of_week) = DayOfWeek::parser().parse(command_text).unwrap();

                let (_, canteen) =
                    opt(peek(|input| Canteen::parser().parse(input)))(args_text.trim())
                        .map_err(|e| ParseError::Custom(e.to_owned().into()))?;

                Ok(Command::Daily(DailyArgs {
                    day_of_week,
                    canteen,
                }))
            }
        }
    }
}

fn parse_cancel(input: &str) -> IResult<&str, internal::Command> {
    let (input, _) = tag_no_case("cancel")(input)?;

    Ok((input, internal::Command::Cancel))
}

fn parse_daily(input: &str) -> IResult<&str, internal::Command> {
    let (input, _) = peek(|input| DayOfWeek::parser().parse(input))(input)?;

    Ok((input, internal::Command::Daily))
}
mod internal {
    pub enum Command {
        Cancel,
        Daily,
    }
}

#[cfg(test)]
mod test {
    use crate::{model::DayOfWeek, tg::command::DailyArgs};

    use super::Command;

    #[test]
    fn parse_command_with_botname() {
        let command_text = "/heute@mybotname";
        let parsed = Command::parse(command_text, "mybotname");

        assert!(parsed.is_ok());

        assert_eq!(
            parsed.unwrap(),
            Command::Daily(DailyArgs {
                day_of_week: DayOfWeek::Today,
                canteen: None
            })
        );
    }
}

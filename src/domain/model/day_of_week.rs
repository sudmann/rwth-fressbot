use chrono::Datelike;
use strum_macros::EnumIter;

#[derive(Clone, Copy, Debug, PartialEq, Eq, EnumIter)]
pub enum DayOfWeek {
    Today,
    Tomorrow,
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
}

impl DayOfWeek {
    pub fn parser() -> parser::DayOfWeekParser {
        parser::DayOfWeekParser
    }
}

impl From<DayOfWeek> for chrono::NaiveDate {
    fn from(dow: DayOfWeek) -> Self {
        let now = chrono::Local::now();
        let dow: chrono::Weekday = dow.into();

        // cast: result is non-negative
        let offset: u64 = {
            let w1 = now.weekday().num_days_from_monday() as i32;
            let w2 = dow.num_days_from_monday() as i32;

            let mut res = (w2 - w1) % 7;
            // res is neg if w2-w1 is neg: https://doc.rust-lang.org/std/primitive.i32.html#impl-Rem%3Ci32%3E-for-i32
            if res < 0 {
                res += 7;
            }
            res
        } as u64;

        now.date_naive() + chrono::Days::new(offset)
    }
}

impl From<DayOfWeek> for chrono::Weekday {
    fn from(dow: DayOfWeek) -> Self {
        match dow {
            DayOfWeek::Today => chrono::Local::now().weekday(),
            DayOfWeek::Tomorrow => (chrono::Local::now() + chrono::Days::new(1)).weekday(),
            DayOfWeek::Monday => Self::Mon,
            DayOfWeek::Tuesday => Self::Tue,
            DayOfWeek::Wednesday => Self::Wed,
            DayOfWeek::Thursday => Self::Thu,
            DayOfWeek::Friday => Self::Fri,
        }
    }
}

pub(super) mod parser {
    use super::DayOfWeek;
    use nom::{branch::alt, bytes::complete::tag_no_case, IResult};

    type ParseResult<'a> = IResult<&'a str, DayOfWeek>;

    pub struct DayOfWeekParser;

    impl DayOfWeekParser {
        pub fn parse<'a>(&self, input: &'a str) -> ParseResult<'a> {
            parse_day_of_week(input)
        }
    }

    pub fn parse_day_of_week(input: &str) -> ParseResult<'_> {
        alt((
            today, tomorrow, monday, tuesday, wednesday, thursday, friday,
        ))(input)
    }

    fn today(input: &str) -> ParseResult<'_> {
        let (input, _) = alt((tag_no_case("heute"), tag_no_case("hoide")))(input)?;

        Ok((input, DayOfWeek::Today))
    }

    fn tomorrow(input: &str) -> ParseResult<'_> {
        let (input, _) = tag_no_case("morgen")(input)?;

        Ok((input, DayOfWeek::Tomorrow))
    }

    fn monday(input: &str) -> ParseResult<'_> {
        let (input, _) = tag_no_case("montag")(input)?;

        Ok((input, DayOfWeek::Monday))
    }

    fn tuesday(input: &str) -> ParseResult<'_> {
        let (input, _) = alt((tag_no_case("dienstag"), tag_no_case("schnitzeldienstag")))(input)?;

        Ok((input, DayOfWeek::Tuesday))
    }

    fn wednesday(input: &str) -> ParseResult<'_> {
        let (input, _) = alt((tag_no_case("mittwoch"), tag_no_case("mettwoch")))(input)?;

        Ok((input, DayOfWeek::Wednesday))
    }

    fn thursday(input: &str) -> ParseResult<'_> {
        let (input, _) = alt((tag_no_case("donnerstag"), tag_no_case("vizefreitag")))(input)?;

        Ok((input, DayOfWeek::Thursday))
    }

    fn friday(input: &str) -> ParseResult<'_> {
        let (input, _) = tag_no_case("freitag")(input)?;

        Ok((input, DayOfWeek::Friday))
    }
}

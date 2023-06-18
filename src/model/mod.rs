mod canteen;
mod day_of_week;
pub mod menu;

pub use canteen::Canteen;
pub use day_of_week::DayOfWeek;

pub mod parse {
    pub use super::canteen::parser::{parse as parse_canteen, CanteenParser};
    pub use super::day_of_week::parser::{parse_day_of_week, DayOfWeekParser};
}

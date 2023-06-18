use itertools::Itertools;
use std::{
    collections::HashMap,
    fmt::{self, Write},
};

use strum_macros::{Display, EnumIter, IntoStaticStr};

#[derive(Debug, Clone)]
pub struct Menu {
    dishes: HashMap<String, Vec<Dish>>,
    extras: Vec<MenuExtra>,
}

impl Menu {
    pub fn new<E: Into<MenuExtra>>(dishes: HashMap<String, Vec<Dish>>, extras: Vec<E>) -> Self {
        Self {
            dishes,
            extras: extras.into_iter().map(Into::<MenuExtra>::into).collect(),
        }
    }

    pub fn fmt_html(&self) -> Result<String, fmt::Error> {
        let mut s = String::new();
        for (n, (categ, dishes)) in self
            .dishes
            .iter()
            .sorted_by(|(k1, _), (k2, _)| k1.cmp(k2))
            .enumerate()
        {
            if dishes.is_empty() {
                continue;
            }

            let emoji = match categ.as_str() {
                "Klassiker" => "🍴",
                "Vegetarisch" => "🥦",
                "Tellergericht" => "🍲",
                "Burger" => "🍔",
                "Wok" => "🥡",
                "Pizza" => "🍕",
                _ => "",
            };

            write!(s, "<em>{categ}</em>")?;
            if !emoji.is_empty() {
                write!(s, " {emoji}")?;
            }
            write!(s, "\n")?;

            for dish in dishes {
                let dish_md = dish.fmt_html()?;
                write!(s, "{dish_md}\n")?;
            }

            if n + 1 < self.dishes.len() {
                write!(s, "\n")?;
            }
        }

        for extras in self.extras.iter() {
            let extras_html = extras.fmt_html()?;
            write!(s, "\n{extras_html}")?;
        }

        Ok(s)
    }
}

#[derive(Debug, Clone)]
pub struct MenuExtra {
    category: String,
    extra: String,
}

impl MenuExtra {
    pub fn new(category: String, extra: String) -> Self {
        Self { category, extra }
    }

    pub fn fmt_html(&self) -> Result<String, std::fmt::Error> {
        let mut s = String::new();
        write!(s, "<em>{}</em>: {}", self.category, self.extra)?;
        Ok(s)
    }
}

impl<S: Into<String>> From<(S, S)> for MenuExtra {
    fn from(value: (S, S)) -> Self {
        Self::new(value.0.into(), value.1.into())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dish {
    name: String,
    ingreds: Vec<String>,
    labels: Vec<Label>,
    price: String,
}

impl Dish {
    pub fn new(name: String, descs: Vec<String>, labels: Vec<Label>, price: String) -> Self {
        Self {
            name,
            ingreds: descs,
            labels,
            price,
        }
    }

    pub fn fmt_html(&self) -> Result<String, fmt::Error> {
        let mut html = String::new();
        write!(html, "<strong>{}</strong>", self.name)?;
        if !self.ingreds.is_empty() {
            write!(html, " | ")?;
        }

        write!(html, "{}", self.ingreds.join(", "))?;

        if !self.labels.is_empty() {
            let label_emoj: Vec<_> = self.labels.iter().map(|l| format!("{l}")).collect();
            write!(html, " {}", label_emoj.join(" "))?;
        }

        write!(html, " – <strong>{}</strong>", self.price)?;

        Ok(html)
    }
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, EnumIter, IntoStaticStr)]
pub enum Category {
    #[strum(serialize = "Burger Classics")]
    BurgerClassic,
    #[strum(serialize = "Burger der Woche")]
    BurgerWeekly,
    #[strum(serialize = "Klassiker")]
    Classic,
    Pasta,
    #[strum(serialize = "Pizza Classics")]
    PizzaClassic,
    #[strum(serialize = "Pizza des Tages")]
    PizzaDaily,
    #[strum(serialize = "Tellergericht")]
    PlateDish,
    #[strum(serialize = "Vegetarisch")]
    Veggie,
    Wok,
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, EnumIter, IntoStaticStr)]
pub enum Label {
    #[strum(serialize = "🐮")]
    Beef,
    #[strum(serialize = "🐔")]
    Chicken,
    #[strum(serialize = "🐟")]
    Fish,
    #[strum(serialize = "🐷")]
    Pork,
    #[strum(serialize = "🥑")]
    Vegan,
    #[strum(serialize = "🌱")]
    Veggie,
}

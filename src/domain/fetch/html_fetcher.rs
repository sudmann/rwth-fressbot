use std::collections::HashMap;

use anyhow::anyhow;
use chrono::NaiveDate;
use scraper::{ElementRef, Html};
use strum::EnumCount;

use crate::domain::model::{
    menu::{Dish, Label, Menu, MenuExtra},
    Canteen,
};

use super::err::FetcherError;

fn menu_url(canteen: Canteen) -> &'static str {
    lazy_static! {
        static ref URLS: [&'static str; Canteen::COUNT] = init_menu_urls();
    }

    fn init_menu_urls() -> [&'static str; Canteen::COUNT] {
        fn format_url(s: &str) -> String {
            format!(
                "https://www.studierendenwerk-aachen.de/speiseplaene/{}-w.html",
                s
            )
        }

        fn string_to_static_str(s: String) -> &'static str {
            Box::leak(s.into_boxed_str())
        }

        let arr = [
            "academica",
            "ahornstrasse",
            "bayernallee",
            "templergraben",
            "eupenerstrasse",
            "juelich",
            "kmac",
            "suedpark",
            "vita",
        ]
        .map(|s: &str| string_to_static_str(format_url(s)));

        arr
    }

    let idx = match canteen {
        Canteen::Academica => 0,
        Canteen::Ahorn => 1,
        Canteen::Bayernallee => 2,
        Canteen::Bistro => 3,
        Canteen::Eupener => 4,
        Canteen::Jülich => 5,
        Canteen::KMAC => 6,
        Canteen::Süd => 7,
        Canteen::Vita => 8,
    };

    URLS[idx]
}

#[derive(Debug, Clone)]
pub struct HtmlMenuFetcher {
    http: reqwest::Client,
}

impl HtmlMenuFetcher {
    pub fn new() -> Self {
        Self::with_client(reqwest::Client::new())
    }

    pub fn with_client(client: reqwest::Client) -> Self {
        Self { http: client }
    }

    pub async fn fetch_daily_menu(
        &self,
        day: chrono::NaiveDate,
        canteen: Canteen,
    ) -> anyhow::Result<Menu> {
        let menu_html = self.fetch_html(menu_url(canteen)).await?;

        let matching_menu_container = menu_html
            .select(&selectors::DAILY_MENU_WRAPPER)
            .filter(|elm| {
                elm.select(&selectors::DATE_TITLE)
                    .flat_map(|elm| elm.text())
                    .next()
                    .and_then(|text| re::DATE_REGEX.find(text))
                    .and_then(|m| NaiveDate::parse_from_str(m.as_str(), "%d.%m.%Y").ok())
                    .map(|section_date| section_date == day)
                    .unwrap_or(false)
            })
            .flat_map(|e| e.children())
            .filter_map(|node| ElementRef::wrap(node))
            .filter(|e| selectors::DIV.matches(e))
            .next()
            .ok_or(FetcherError::CanteenClosed {
                canteen: canteen,
                date: day,
            })?;

        self.parse_menu(matching_menu_container)
    }

    async fn fetch_html(&self, url: &str) -> anyhow::Result<Html> {
        let resp_text = self.http.get(url).send().await?.text().await?;
        Ok(Html::parse_document(&resp_text))
    }

    fn parse_menu(&self, container: ElementRef) -> anyhow::Result<Menu> {
        let table_elms = container
            .children()
            .filter_map(|e| ElementRef::wrap(e))
            .filter(|e| {
                let e = e.value();
                e.name().to_ascii_lowercase() == "table"
            });

        let (menu_table, extras_table) = table_elms.fold((None, None), |(menus, extras), e| {
            let elm = e.value();
            if elm.has_class("menues", scraper::CaseSensitivity::AsciiCaseInsensitive) {
                (Some(e), extras)
            } else if elm.has_class("extras", scraper::CaseSensitivity::AsciiCaseInsensitive) {
                (menus, Some(e))
            } else {
                (menus, extras)
            }
        });

        let menu_table = menu_table.ok_or(FetcherError::ElementNotFound {
            tag: "table".to_owned(),
            cls: vec!["menues".to_owned()],
        })?;

        let extras_table = extras_table.ok_or(FetcherError::ElementNotFound {
            tag: "table".to_owned(),
            cls: vec!["extras".to_owned()],
        })?;

        let dishes = self.parse_menu_table(menu_table)?;
        let extras = self.parse_extras_table(extras_table)?;

        Ok(Menu::new(dishes, extras))
    }

    fn parse_menu_table(&self, table: ElementRef) -> anyhow::Result<HashMap<String, Vec<Dish>>> {
        lazy_static! {
            static ref ROW: scraper::Selector = scraper::Selector::parse("tbody > tr").unwrap();
        }

        let dishes: anyhow::Result<HashMap<String, Vec<Dish>>> = table
            .select(&ROW)
            .map(|tr| self.parse_menu_dish(tr))
            .fold(Ok(HashMap::new()), |acc, val| {
                let (cat, dish) = val?;
                let mut map = acc?;
                map.entry(cat).or_insert(vec![]).push(dish);
                Ok(map)
            });

        dishes
    }

    fn parse_menu_dish(&self, tr: ElementRef) -> anyhow::Result<(String, Dish)> {
        lazy_static! {
            static ref CATEGORY: scraper::Selector =
                scraper::Selector::parse("span.menue-category").unwrap();
            static ref DISH_DESCR: scraper::Selector =
                scraper::Selector::parse("span.menue-desc > .expand-nutr").unwrap();
            static ref PRICE: scraper::Selector =
                scraper::Selector::parse("span.menue-price").unwrap();
        }

        let category = tr
            .select(&CATEGORY)
            .next()
            .ok_or(anyhow!("No span with class \"menue-category\""))?
            .text()
            .next()
            .ok_or(anyhow!(".menu-category contains no text node"))?
            .trim()
            .split_ascii_whitespace()
            .next()
            // unwrap: split yields at least one element
            .unwrap();

        let dish: String = tr
            .select(&DISH_DESCR)
            .next()
            .ok_or(anyhow!("No span with class \"menue-descr\""))?
            .children()
            .filter_map(|node| node.value().as_text())
            .map(|txt_node| <str as AsRef<str>>::as_ref(txt_node))
            .collect();

        let mut dish_iter = dish.split("|").map(|s| s.trim());

        // unwrap: split yields at least one element
        let dish_name = dish_iter.next().unwrap();

        let dish_descs: Vec<String> = dish_iter.map(|s| s.to_owned()).collect();

        let price = tr
            .select(&PRICE)
            .next()
            .and_then(|elm| elm.text().next())
            .map(|text| text.to_owned());

        let labels: Vec<_> = tr
            .value()
            .classes()
            .filter_map(|cls| match cls {
                "Fisch" => Some(Label::Fish),
                "OLV" => Some(Label::Veggie),
                "vegan" => Some(Label::Vegan),
                "Geflügel" => Some(Label::Chicken),
                "Schwein" => Some(Label::Pork),
                "Rind" => Some(Label::Beef),
                _ => None,
            })
            .collect();

        Ok((
            category.to_owned(),
            Dish::new(dish_name.to_owned(), dish_descs, labels, price),
        ))
    }

    fn parse_extras_table(&self, table: ElementRef) -> anyhow::Result<Vec<MenuExtra>> {
        lazy_static! {
            static ref ROW: scraper::Selector =
                scraper::Selector::parse("tbody tr .menue-wrapper").unwrap();
            static ref EXTRA_CELL: scraper::Selector =
                scraper::Selector::parse("span.menue-item.extra").unwrap();
        }

        table
            .select(&ROW)
            .map(|td| -> anyhow::Result<MenuExtra> {
                let cells = td.select(&EXTRA_CELL);

                let category = cells
                    .clone()
                    .filter(|elm| {
                        elm.value().has_class(
                            "menue-category",
                            scraper::CaseSensitivity::AsciiCaseInsensitive,
                        )
                    })
                    .next()
                    .and_then(|elm| elm.text().next())
                    .map(|text| text.trim())
                    // Fallback to the empty string as a category if none is given
                    .unwrap_or("")
                    .to_owned();

                let extras = cells
                    .clone()
                    .filter(|elm| {
                        elm.value()
                            .has_class("menue-desc", scraper::CaseSensitivity::AsciiCaseInsensitive)
                    })
                    .next()
                    .ok_or(anyhow!("No span with class \"menue-desc\""))?
                    .children()
                    .filter_map(|node| node.value().as_text())
                    .map(|txt_node| txt_node.trim().to_string())
                    .collect::<Vec<_>>();

                let extras = match extras.len().checked_sub(1).map(|idx| extras.split_at(idx)) {
                    None => extras.join(""),
                    Some((init, last)) => init.join(", ") + " oder " + &last[0],
                };

                Ok(MenuExtra::new(category, extras))
            })
            .collect()
    }
}

pub mod selectors {
    use scraper::Selector;

    lazy_static! {
        pub static ref DAILY_MENU_WRAPPER: Selector =
            Selector::parse("body div.accordion > div").unwrap();
        pub static ref DATE_TITLE: Selector = Selector::parse("h3 > a").unwrap();
        pub static ref DIV: Selector = Selector::parse("div").unwrap();
    }
}

pub mod re {
    use regex::Regex;

    lazy_static! {
        pub static ref DATE_REGEX: Regex = Regex::new(r"(\d{2}\.\d{2}\.\d{4})").unwrap();
    }
}

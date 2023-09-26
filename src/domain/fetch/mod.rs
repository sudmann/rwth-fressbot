mod html_fetcher;
pub use html_fetcher::HtmlMenuFetcher;

pub mod err {
    use chrono::NaiveDate;
    use thiserror::Error;

    use crate::domain::model::Canteen;

    #[derive(Debug, Clone, Error)]
    pub enum FetcherError {
        #[error("canteen {canteen} is closed on date {}", .date.format("%Y-%m-%d"))]
        CanteenClosed { canteen: Canteen, date: NaiveDate },

        #[error("No element {tag} with class(es) {:?}", &.cls[..])]
        ElementNotFound { tag: String, cls: Vec<String> },
    }
}

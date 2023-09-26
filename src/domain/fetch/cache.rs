use lru::LruCache;

use crate::domain::model::{Canteen, Menu};
use std::sync::{Arc, Mutex};

use super::HtmlMenuFetcher;

const DEFAULT_CACHE_SIZE: usize = 16;

#[derive(Debug, Clone)]
pub struct HtmlMenuFetcherWithCache {
    cache: Arc<Mutex<LruCache<(Canteen, chrono::NaiveDate), CacheEntry<Menu>>>>,
    fetcher: HtmlMenuFetcher,
    cache_fresh_dur: std::time::Duration,
}

impl HtmlMenuFetcherWithCache {
    pub fn new() -> Self {
        let cache = LruCache::new(DEFAULT_CACHE_SIZE.try_into().unwrap());

        Self {
            cache: Arc::new(Mutex::new(cache)),
            fetcher: HtmlMenuFetcher::new(),
            cache_fresh_dur: std::time::Duration::from_secs(10 * 60),
        }
    }

    pub async fn fetch_daily_menu(
        &self,
        day: chrono::NaiveDate,
        canteen: Canteen,
    ) -> anyhow::Result<Menu> {
        let cached_result = self
            .cache
            .lock()
            .map_err(|e| {
                log::warn!("Can not access cache: {}", e.to_string());
                e
            })
            .ok()
            .and_then(|mut cache| {
                let cache_entry = cache.get(&(canteen, day))?;

                log::info!("Result for ({}, {}) is cached", &canteen, &day);

                if cache_entry.is_stale() {
                    let expired_at = cache_entry.created + cache_entry.fresh_dur;
                    log::info!(
                        "Cache entry for ({}, {}) is stale. Expired at {:?} ({} s ago)",
                        &canteen,
                        &day,
                        expired_at,
                        expired_at.elapsed().as_secs()
                    );
                    None
                } else {
                    Some(cache_entry.get_val().clone())
                }
            });

        match cached_result {
            Some(menu) => Ok(menu),
            None => self.fetch_and_insert(day, canteen).await,
        }
    }

    async fn fetch_and_insert(
        &self,
        day: chrono::NaiveDate,
        canteen: Canteen,
    ) -> anyhow::Result<Menu> {
        let menu = self.fetcher.fetch_daily_menu(day, canteen).await?;

        self.cache
            .lock()
            .map_err(|e| {
                log::warn!("Can not access cache: {}", e.to_string());
                e
            })
            .ok()
            .and_then(|mut cache| {
                let entry = CacheEntry {
                    val: menu.clone(),
                    created: std::time::Instant::now(),
                    fresh_dur: self.cache_fresh_dur,
                };

                cache.put((canteen, day), entry)
            });

        Ok(menu)
    }
}

impl Default for HtmlMenuFetcherWithCache {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, PartialEq, Hash, Clone)]
struct CacheEntry<V> {
    val: V,
    created: std::time::Instant,
    fresh_dur: std::time::Duration,
}

impl<V> CacheEntry<V> {
    fn is_fresh(&self) -> bool {
        self.created.elapsed() <= self.fresh_dur
    }

    fn is_stale(&self) -> bool {
        !self.is_fresh()
    }

    fn get_val(&self) -> &V {
        &self.val
    }

    fn get_val_mut(&mut self) -> &mut V {
        &mut self.val
    }
}

mod builder {}

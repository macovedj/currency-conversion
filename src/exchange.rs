use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};
use std::error::Error;
use serde::{Deserialize};
use hyper::Client;
use hyper::body::{self, HttpBody};
use super::Result;

static EXCH_URL: &str = "https://open.er-api.com/v6/latest/USD";

#[derive(Debug, Deserialize)]
struct Rates {
    time_next_update_unix: u64,
    rates: HashMap<String, f64>,
}

#[derive(Clone)]
pub struct RatesCache(Arc<RwLock<Option<Rates>>>);

impl RatesCache {
    pub fn new() -> RatesCache {
        RatesCache(Arc::new(RwLock::new(None)))
    }

    pub async fn get_exchange_rate<'a, 'b, T, B>(
        &'a self,
        client: Client<T, B>,
        from: &'b str,
        to: &'b str,
    ) -> Result<Option<f64>>
    where
        T: hyper::client::connect::Connect + Clone + Send + Sync + 'static,
        B: HttpBody + Send + Default + 'static,
        B::Data: Send,
        B::Error: Into<Box<(dyn Error + Sync + Send + 'static)>> + Send,
    {
        // if cached rates are still current, then use those and return calculated exchange rate
        if let Some(rates) = self.0.read().unwrap().as_ref() {
            if SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
                < rates.time_next_update_unix
            {
                eprintln!("rate {}{}: from cache", to, from);
                return match (rates.rates.get(from), rates.rates.get(to)) {
                    (Some(from), Some(to)) => Ok(Some(to / from)),
                    _ => Ok(None),
                };
            }
        }

        // get updated rates
        let rates: Rates =
            serde_json::from_slice(&body::to_bytes(client.get(EXCH_URL.parse()?).await?).await?)?;

        eprintln!("rate {}{}: making request", to, from);

        // calculate the exchange rate
        let rate = match (rates.rates.get(from), rates.rates.get(to)) {
            (Some(from), Some(to)) => Ok(Some(to / from)),
            _ => Ok(None),
        };

        // update cache
        *self.0.write().unwrap() = Some(rates);

        rate
    }
}
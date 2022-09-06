use std::error::Error;
use std::task::{Context, Poll};
use futures::future::BoxFuture;
use hyper::service::{Service};
use hyper::{Body, Request, Response};
use super::{country, exchange, server};

pub struct CurrencyService {
  country_cache: country::CountryCache,
  rates_cache: exchange::RatesCache,
}

impl CurrencyService {
  pub fn new(
    country_cache: country::CountryCache,
    rates_cache: exchange::RatesCache
  ) -> CurrencyService {
      CurrencyService {
        country_cache,
        rates_cache,
      }
  }
}

impl Service<Request<Body>> for CurrencyService {
  type Response = Response<Body>;
  type Error = Box<dyn Error + Send + Sync>;
  type Future = BoxFuture<'static, std::result::Result<Self::Response, Self::Error>>;

  fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<std::result::Result<(), Self::Error>> {
      Poll::Ready(Ok(()))
  }

  fn call(&mut self, req: Request<Body>) -> Self::Future {
      let country_cache = self.country_cache.clone();
      let rates_cache = self.rates_cache.clone();
      Box::pin(server::handler(rates_cache, country_cache, req))
  }
}
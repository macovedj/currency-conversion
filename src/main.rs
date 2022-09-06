use std::convert::Infallible;
use std::net::SocketAddr;
use std::error::Error;
use hyper::{Server};
use hyper::service::{make_service_fn};
mod country;
mod exchange;
mod service;
mod server;

type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

#[tokio::main]
async fn main() {
    // We'll bind to 127.0.0.1:3000
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    let country_cache = Box::leak(Box::new(country::CountryCache::new()));
    let rates_cache = Box::leak(Box::new(exchange::RatesCache::new()));
    // A `Service` is needed for every connection, so this
    // creates one from our `hello_world` function.
    let make_svc = make_service_fn(|_conn| async {
      let svc = service::CurrencyService::new(
        country_cache.clone(), 
        rates_cache.clone(),
      );
      // service_fn converts our function into a `Service`
      Ok::<_, Infallible>(svc)
    });

    let mut svc = tower::ServiceBuilder::new()
        .rate_limit(100, std::time::Duration::from_millis(200))
        .service(make_svc);

    let server = Server::bind(&addr).serve(svc);

    // Run this server for... forever!
    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}


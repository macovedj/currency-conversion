use std::fmt;
use futures::try_join;
use hyper::body;
use hyper::client::HttpConnector;
use futures::FutureExt;
use hyper::{Body, Request, Response};
use hyper::header::{CACHE_CONTROL, HeaderValue};
use hyper::{Method, StatusCode};
use hyper::Client;
use hyper_tls::HttpsConnector;
use serde::{Deserialize, Serialize};
use serde_json;
use super::{exchange, country, Result};

#[derive(Debug, Deserialize, Serialize)]
struct ApiBody {
    from: String,
    to: String,
    amount: f64,
}

#[derive(Debug, Deserialize, Serialize)]
struct Conversion {
  from: String,
  to: String,
  amount: f64
}

impl fmt::Display for Conversion {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
      write!(f, "{{from: {}, to: {}, amount: {}}}", self.from, self.to, self.amount)
  }
}

async fn get_conversion(
  req: Request<Body>,
  country_cache: country::CountryCache,
  rates_cache: exchange::RatesCache,
  res: &mut Response<Body>
) -> 
  Result<()> {
  let https = HttpsConnector::new();
  let client = Client::builder().build::<_, hyper::Body>(https);
  let req_body: ApiBody =
    match serde_json::from_slice(&body::to_bytes(req.into_body()).await?) {
        Ok(req_body) => req_body,
        Err(err) => {
            let mut res = Response::new(Body::from(format!("{}", err)));
            *res.status_mut() = StatusCode::BAD_REQUEST;
            ApiBody {
              from: String::from(""),
              to: String::from(""),
              amount: 0.0
            }
        }
    };
    let get_from_currency = country_cache.get_currency(client.clone(), String::from(&req_body.from), |s, c, cl| country::country_requester(s, c, cl).boxed());
    let get_to_currency = country_cache.get_currency(client.clone(), String::from(&req_body.to), |s, c, cl| country::country_requester(s, c, cl).boxed());
    
    let (origin, destination) = match try_join!(get_from_currency, get_to_currency) {
      Ok((Some(origin), Some(destination))) => (origin, destination),
      Ok((None, _)) => {
          *res.status_mut() = StatusCode::NOT_FOUND;
          *res.body_mut() = Body::from(format!(
            "cannot find country: {}",
            req_body.from
          ));
        return Ok(())
      }
      Ok((_, None)) => {
          *res.status_mut() = StatusCode::NOT_FOUND;
          *res.body_mut() = Body::from(format!(
            "cannot find country: {}",
            req_body.to
          ));
          return Ok(());
      }
      Err(err) => {
          *res.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
          *res.body_mut() = Body::from(format!(
            "cannot find country: {}",
            req_body.from
          ));
          return Err(err);
      }
    };

    let mut conversions = Vec::new();
    for orig in &origin {
      for dest in &destination {
        let rate = match rates_cache.get_exchange_rate(client.clone(), &orig, &dest).await {
          Ok(Some(rate)) => rate,
          Ok(None) => {
              *res.status_mut() = StatusCode::NOT_FOUND;
              *res.body_mut() = Body::from(format!(
                  "cannot find exchange rate: {}{}",
                  orig, dest
              ));
              return Ok(());
          }
          Err(err) => {
              *res.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
              *res.body_mut() = Body::from(format!(
                  "cannot find exchange rate: {}{}",
                  orig, dest
              ));
              return Ok(());
          }
        };
        conversions.push(Conversion {
          from: String::from(orig),
          to: String::from(dest),
          amount: rate * req_body.amount
        });
      }
    }
    let json = serde_json::to_string(&conversions).unwrap();
    *res.body_mut() = Body::from(json);
    return Ok(())
}
// let server = Server::bind(&addr).serve(make_svc);
pub async fn handler(rates_cache: exchange::RatesCache, country_cache: country::CountryCache, req: Request<Body>) -> Result<Response<Body>> {
  let mut response = Response::new(Body::empty());
  response.headers_mut().insert(CACHE_CONTROL, HeaderValue::from_static("86400"));

  match (req.method(), req.uri().path()) {
      (&Method::POST, "/currency") => {
        get_conversion(req, country_cache, rates_cache, &mut response).await?;
      },
      _ => {
        *response.status_mut() = StatusCode::NOT_FOUND;
        let mut res = Response::new(Body::empty());
        *res.status_mut() = StatusCode::NOT_FOUND;
      },
  };

  Ok(response)
}
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::error::Error;
use hyper::client::HttpConnector;
use hyper::{Client};
use hyper::body::{self, HttpBody};
use hyper::{StatusCode};
use hyper_tls::HttpsConnector;
use serde::{Deserialize, Serialize};
use super::Result;

const REST_COUNTRIES_BASE_URL: &str = "https://restcountries.com/v3.1/name/";

#[derive(Default, Debug, Deserialize, Serialize, Clone)]
struct CountryResp {
  currencies: HashMap<String,Currency>
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Currency {
  name: String
}

#[derive(Clone)]
pub struct CountryCache (Arc<RwLock<HashMap<String, Vec::<String>>>>);

impl CountryCache {
  pub fn new() -> Self {
    CountryCache(Arc::new(RwLock::new(HashMap::new())))
  }

  pub async fn get_currency<'a, 'b, T, B,
  F : FnOnce(String, &'a CountryCache, Client<T, B>) -> futures::future::BoxFuture<Result<Option<Vec<String>>>>
  >(
    &'a self,
    client: Client<T, B>,
    country: String,
    requester: F
) 
-> Result<Option<Vec<String>>>
where
    T: hyper::client::connect::Connect + Clone + Send + Sync + 'static,
    B: HttpBody + Send + Default + 'static,
    B::Data: Send,
    B::Error: Into<Box<(dyn Error + Sync + Send + 'static)>> + Send,
{
  let country = country.to_lowercase();
  let res = requester(country, &self, client).await;
  return res
  }
}

pub async fn country_requester(country: String, cache: &CountryCache, client: Client<HttpsConnector<HttpConnector>>) -> Result<Option<Vec<String>>> {
    let country = country.to_lowercase();
    // check cache to see if country currency names are already known and is so respond w cache data
    if let Some(currencies) = cache.0.read().unwrap().get(&country) {
        eprintln!("country {}: from cache", &country);
        return Ok(Some(currencies.to_owned()));
    }

    eprintln!("country {}: making request", country);
    let uri = format!("{}{}", REST_COUNTRIES_BASE_URL, country).parse()?;
    let res = client.get(uri).await?;
    if res.status() == StatusCode::NOT_FOUND {
      eprintln!("NOT FOUND");
        return Ok(None);
    }
    let convs: Vec<CountryResp> = serde_json::from_slice(&body::to_bytes(res).await?)?;

    // construct vector of country currency names (they are keys in exchange api response)
    let mut currencies = Vec::new();
    for conv in convs.clone() {
      conv.currencies.keys().next().map(|cur| {
        currencies.push(String::from(cur));
      });
    }

    // write response data into cache before returning
    cache.0.write().unwrap().insert(country.clone(), currencies);
    Ok(convs.iter().map(|conv| {
      conv.currencies.keys().next().map(|cur| {
        cur.to_owned()
      })
    }).collect())
}

pub async fn test_request(country: String, cache: &CountryCache, _client: Client<HttpsConnector<HttpConnector>>) -> Result<Option<Vec<String>>> {
  let country = country.to_lowercase();
    // check cache to see if country currency names are already known and is so respond w cache data
    if let Some(currencies) = cache.0.read().unwrap().get(&country) {
        eprintln!("country {}: from cache", &country);
        return Ok(Some(currencies.to_owned()));
    }

    eprintln!("country {}: making request", country);
    let mut convs = Vec::new();
    convs.push(String::from("HKD"));
    convs.push(String::from("TWD"));
    convs.push(String::from("CNY"));
    convs.push(String::from("MOP"));

    // write response data into cache before returning
    cache.0.write().unwrap().insert(country.clone(), convs.clone());
    Ok(Some(convs))
}

#[cfg(test)]
mod test {
    use crate::country::test_request;

    use super::{CountryCache, Client};
    use hyper_tls::HttpsConnector;
    use hyper;
    use futures::FutureExt;

    #[tokio::test]
    async fn country_cache() {
      let cache = CountryCache::new();
      let https = HttpsConnector::new();
      let client = Client::builder().build::<_, hyper::Body>(https);

      // two distinct inputs yields two values in the cache
      cache.get_currency(client.clone(), String::from("china"), |s, c, cl| test_request(s,c,cl).boxed()).await;
      cache.get_currency(client.clone(), String::from("china"), |s, c, cl| test_request(s,c,cl).boxed()).await;
      cache.get_currency(client.clone(), String::from("france"), |s, c, cl| test_request(s,c,cl).boxed()).await;
      assert_eq!(cache.0.read().unwrap().keys().len(), 2);
    }
    // everything else the same
}
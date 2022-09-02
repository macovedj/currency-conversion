use std::collections::HashMap;
use std::convert::Infallible;
use std::fmt;
use std::net::SocketAddr;
use hyper::{Body, Request, Response, Server, Uri};
use hyper::service::{make_service_fn, service_fn};
// use reqwest;
use hyper::{Method, StatusCode};
use futures::TryStreamExt as _;
use hyper::Client;
use hyper::body;
use hyper_tls::HttpsConnector;
use serde::{Deserialize, Serialize};
use serde_json::{Value};

#[derive(Debug, Deserialize, Serialize)]
struct Conversion {
  from: String,
  to: String,
  amount: f32
}

#[derive(Debug, Deserialize, Serialize)]
struct Currency {
  name: String
}

#[derive(Default, Debug, Deserialize, Serialize)]
struct CountryResp {
  currencies: HashMap<String,Currency>,
}

#[derive(Default, Debug, Deserialize, Serialize)]
struct Exchange {
  conversion_rate: f32,
}

impl fmt::Display for Conversion {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
      write!(f, "{{from: {}, to: {}, amount: {}}}", self.from, self.to, self.amount)
  }
}

async fn hello_world(req: Request<Body>) -> Result<Response<Body>,  Box<dyn std::error::Error + Send + Sync>> {
  let mut response = Response::new(Body::empty());

  match (req.method(), req.uri().path()) {
      (&Method::POST, "/currency") => {
        let https = HttpsConnector::new();
        let client = Client::builder()
            .build::<_, hyper::Body>(https);

        let bytes = body::to_bytes(req.into_body()).await?;
        let req_bod = String::from_utf8(bytes.to_vec()).expect("response was not valid utf-8");
        let from_conv: Conversion = serde_json::from_str(&req_bod)?;
        let mut base = String::from("https://restcountries.com/v3.1/name/");
        base.push_str(from_conv.from.as_str());
        let uri = base.parse()?;
        let mut resp = client.get(uri).await?;
        let bytes = body::to_bytes(resp).await?;
        let bod = String::from_utf8(bytes.to_vec()).expect("response was not valid utf-8");
        let conv: Vec::<CountryResp> = serde_json::from_str(&bod).unwrap();
        let mut keys = conv[0].currencies.keys();
        let mut origin = "";
        if let Some(key) = keys.next() {
          origin = key;
        }
        let dest_conv: Conversion = serde_json::from_str(&req_bod)?;
        let mut base = String::from("https://restcountries.com/v3.1/name/");
        base.push_str(dest_conv.to.as_str());
        let uri = base.parse()?;
        let mut resp = client.get(uri).await?;
        let bytes = body::to_bytes(resp).await?;
        let bod = String::from_utf8(bytes.to_vec()).expect("response was not valid utf-8");
        let conv: Vec::<CountryResp> = serde_json::from_str(&bod).unwrap();
        let mut keys = conv[0].currencies.keys();
        let mut dest = "";
        if let Some(key) = keys.next() {
         dest = key;
        }
        let mut base = String::from("https://v6.exchangerate-api.com/v6/11cc4f5fccab5ec79c7d2609/pair/");
        base.push_str(origin);
        base.push_str("/");
        base.push_str(dest);
        let uri = base.parse::<Uri>()?;
        let mut resp = client.get(uri).await?;
        let bytes = body::to_bytes(resp).await?;
        let bod = String::from_utf8(bytes.to_vec()).expect("response was not valid utf-8");
        let conv: Exchange = serde_json::from_str(&bod).unwrap();

        let user_resp = Conversion {
          from: String::from(from_conv.from),
          to: String::from(from_conv.to),
          amount: from_conv.amount * conv.conversion_rate
        };
        *response.body_mut() = Body::from(user_resp.to_string());
      },
      (&Method::POST, "/echo") => {
          println!("DOING SOME ECHOING");
          *response.body_mut() = req.into_body();
      },
      (&Method::POST, "/echo/uppercase") => {
        // This is actually a new `futures::Stream`...
        let mapping = req
            .into_body()
            .map_ok(|chunk| {
                chunk.iter()
                    .map(|byte| byte.to_ascii_uppercase())
                    .collect::<Vec<u8>>()
            });
    
        // Use `Body::wrap_stream` to convert it to a `Body`...
        *response.body_mut() = Body::wrap_stream(mapping);
    },
      _ => {
          *response.status_mut() = StatusCode::NOT_FOUND;
      },
  };

  Ok(response)
}

#[tokio::main]
async fn main() {
    // We'll bind to 127.0.0.1:3000
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    // A `Service` is needed for every connection, so this
    // creates one from our `hello_world` function.
    let make_svc = make_service_fn(|_conn| async {
        // service_fn converts our function into a `Service`
        Ok::<_, Infallible>(service_fn(hello_world))
    });

    let server = Server::bind(&addr).serve(make_svc);

    // Run this server for... forever!
    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}


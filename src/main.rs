use lazy_static::lazy_static;
use serde_json::Value;
use warp::http::Response;
use warp::Filter;

use futures::future::join;
use std::collections::HashMap;
use std::sync::Mutex;
use tokio::time::interval;
use tokio::time::Duration;

lazy_static! {
    static ref STORE: Mutex<HashMap<String, Value>> = {
        let m = HashMap::new();
        Mutex::new(m)
    };
    static ref SYMBOLS: Mutex<Vec<&'static str>> = {
        let m = vec!["ATOM", "IRIS", "LUNA", "BAND", "CRO", "AKT", "KAVA", "OKT"];
        Mutex::new(m)
    };
}
#[tokio::main]
async fn main() {
    let t1 = timer();
    let server = server();

    join(t1, server).await;
}

async fn server() {
    let token = warp::path!("token" / String).map(|name: String| {
        let mut symbols = SYMBOLS.lock().unwrap();
        let s: &'static str = Box::leak(name.into_boxed_str());
        symbols.push(s);
        response_text("OK".to_string())
    });
    let quote = warp::path!("quote" / String).map(|name| {
        let map = STORE.lock().unwrap();
        // println!("map: {}=> {}", &name, serde_json::to_string().unwrap());
        match map.get(&name) {
            Some(q) => response_value(q),
            None => response_text("Not found".to_string()),
        }
    });

    warp::serve(token.or(quote)).run(([0, 0, 0, 0], 8000)).await
}

fn response_text(body: String) -> Response<String> {
    response(format!("{{\"result\":{:?}}}", body))
}

fn response_value(body: &Value) -> Response<String> {
    response(format!("{}", body))
}

fn response(body: String) -> Response<String> {
    match Response::builder()
        .header("Access-Control-Expose-Headers", "Content-Length")
        .header("Access-Control-Allow-Origin", "*")
        .header("Content-Type", "application/json; charset=utf-8")
        .body(format!("{}", body))
    {
        Ok(t) => t,
        Err(e) => Response::new(format!("{{\"error\": \"{}\"}}", e.to_string())),
    }
}

async fn timer() {
    let mut interval = interval(Duration::from_secs(60*60 ));

    loop {
        interval.tick().await;
        fetch_coin_market_quotes().await;
    }
}

async fn fetch_coin_market_quotes() {
    let symbols = SYMBOLS.lock().unwrap();
    let client = reqwest::Client::new();
    let result = client
        .get("https://pro-api.coinmarketcap.com/v1/cryptocurrency/quotes/latest")
        .query(&[("symbol", symbols.join(","))])
        //.query(&[("convert","USD,CNY,EUR,JPY,KRW")]) //plan is not supported
        .header("X-CMC_PRO_API_KEY", "2a6f96eb-6a10-429a-8a56-9851ed509cf0")
        .send()
        .await;
    match result {
        Ok(response) => match response.text().await {
            Ok(text) => {
                println!("text {}", &text);
                let json: Value = serde_json::from_str(&text).unwrap();
                if let Some(quotes) = json.get("data") {
                    let mut map = STORE.lock().unwrap();
                    quotes.as_object().iter().for_each(|q_map| {
                        q_map.iter().for_each(|(k, v)| {
                            map.insert(k.to_string(), v.clone());
                        })
                    })
                }
            }
            Err(e) => println!("error:{}", e.to_string()),
        },
        Err(e) => println!("HTTP ERROR: {}", e.to_string()),
    }
}

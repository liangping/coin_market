use lazy_static::lazy_static;
use serde_json::{Value};
use warp::http::Response;
use warp::Filter;

use futures::future::join;
use std::collections::{BTreeMap};
use std::sync::Mutex;
use tokio::time::interval;
use tokio::time::Duration;
use once_cell::sync::Lazy;
use std::fs;

pub static IDS: Lazy<BTreeMap<String, String>> =
    Lazy::new(|| {
        let x = fs::read_to_string("pairs.txt").unwrap();
        let pairs: Vec<&str> = x.split("\n").collect();
        let mut m = BTreeMap::new();
        pairs.iter().for_each(|i| {
            if !i.starts_with("#") {
                let pair: Vec<&str> = i.trim().split(',').collect();
                println!("{:?}", pair);
                if pair.len() >1 {
                    let id = pair.get(0).unwrap();
                    let symbol = pair.get(1).unwrap();
                    if id.len() > 1 && symbol.len() > 1 {
                        m.insert(id.to_string(), symbol.to_uppercase());
                    }
                }
            }
        });
        m
    });


pub static CURRENCIES: Lazy<Vec<&'static str>> = Lazy::new(|| {
    vec!["usd", "cny", "eur", "jpy", "krw", "sgd", "hkd"]
});

lazy_static! {
    static ref STORE: Mutex<BTreeMap<String, BTreeMap<String, f32>>> = {
        let m = BTreeMap::new();
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
    let quote = warp::path!("quote" / String).map(|name: String| {
        let map = STORE.lock().unwrap();
        // println!("map: {}=> {}", &name, serde_json::to_string().unwrap());
        match map.get(name.as_str()) {
            Some(q) => response(format!("{:?}", q)),
            None => response_text("Not found".to_string()),
        }
    });
    let quotes = warp::path!("quotes").map(|| {
        let map = STORE.lock().unwrap();
        response(format!("{:?}", map))
    });

    let paths = quote.or(quotes);
    //paths.or(quotes);

    warp::serve(paths).run(([0, 0, 0, 0], 8000)).await
}

pub fn response_text(body: String) -> Response<String> {
    response(format!("{{\"result\":{:?}}}", body))
}

pub fn response_value(body: &Value) -> Response<String> {
    response(format!("{}", body))
}

pub fn response(body: String) -> Response<String> {
    match Response::builder()
        .header("Access-Control-Expose-Headers", "Content-Length")
        .header("Access-Control-Allow-Origin", "*")
        //.header("Access-Control-Allow-Origin", "https://ping.pub")
        .header("Content-Type", "application/json; charset=utf-8")
        .body(format!("{}", body))
    {
        Ok(t) => t,
        Err(e) => Response::new(format!("{{\"error\": \"{}\"}}", e.to_string())),
    }
}

async fn timer() {
    let mut interval = interval(Duration::from_secs(5*60 ));

    loop {
        interval.tick().await;
        fetch_coin_gecko_quotes().await;
    }
}

async fn fetch_coin_gecko_quotes() {
    println!("fetch coin gecko quotes");
    let client = reqwest::Client::new();
    let result = client
        .get("https://api.coingecko.com/api/v3/simple/price")
        .query(&[("ids", IDS.keys().map(|s| &**s).collect::<Vec<_>>().join(","))])
        .query(&[("vs_currencies", CURRENCIES.join(","))])
        .query(&[("include_24hr_change", "true")])
        .send()
        .await;
    match result {
        Ok(response) => match response.text().await {
            Ok(text) => {
                let well = text.replace("null", "0");
                println!("text {}", &well);
                let json: BTreeMap<String, BTreeMap<String, f32>> = serde_json::from_str(well.to_owned().as_str()).unwrap();
                // if let Some(quotes) = json.get("data") {
                let mut map = STORE.lock().unwrap();

                json.keys().for_each(|k| {
                    if let Some(key) = IDS.get(k) {
                        if let Some(price) = json.get(k) {

                            map.insert(key.to_string(), price.to_owned());
                        }
                    }
                })
            }
            Err(e) => println!("error:{}", e.to_string()),
        },
        Err(e) => println!("HTTP ERROR: {}", e.to_string()),
    }
}

// async fn fetch_coin_market_quotes() {
//     let symbols = SYMBOLS.lock().unwrap();
//     let client = reqwest::Client::new();
//     let result = client
//         .get("https://pro-api.coinmarketcap.com/v1/cryptocurrency/quotes/latest")
//         .query(&[("symbol", symbols.join(","))])
//         //.query(&[("convert","USD,CNY,EUR,JPY,KRW")]) //plan is not supported
//         .header("X-CMC_PRO_API_KEY", "2a6f96eb-6a10-429a-8a56-9851ed509cf0")
//         .send()
//         .await;
//     match result {
//         Ok(response) => match response.text().await {
//             Ok(text) => {
//                 println!("text {}", &text);
//                 let json: HashMap<> = serde_json::from_str(&text).unwrap();
//                 if let Some(quotes) = json.get("data") {
//                     let mut map = STORE.lock().unwrap();
//                     let tokens = IDS.lock().unwrap();
//                     quotes.as_object().iter().for_each(|q_map| {
//                         q_map.iter().for_each(|(k, v)| {
//                             map.insert(String::from(k.as_str()), v.clone());
//                         })
//                     })
//                 }
//             }
//             Err(e) => println!("error:{}", e.to_string()),
//         },
//         Err(e) => println!("HTTP ERROR: {}", e.to_string()),
//     }
// }

// mod model;
// use yahoo_finance::{history, Interval, Timestamped};
// #[tokio::main]
// async fn main(){

//    // model::main().unwrap();
//    let data = history::retrieve_interval("AAPL", Interval::_6mo).await.unwrap();

//    // print out some high numbers!
//    for bar in &data {
//       println!("Apple hit an intraday high of ${:.2} on {}.", bar.high, bar.datetime().format("%b %e %Y"));
//    }
// }
use axum::{extract::Query, http::StatusCode, response::IntoResponse, routing::get, Router};
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
use url::Url;
use polars::prelude::{JsonReader,SerReader};
use std::io::Read;
use serde_json::Value;
#[tokio::main]
async fn main() {
    // Build our application with a single route
    let app = Router::new().route("/get_sise", get(get_sise_handler));

    // Run it with hyper on localhost:3000
    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

#[derive(Deserialize)]
struct GetSiseParams {
    symbol: String,
    start_time: String,
    end_time: String,
    timeframe: Option<String>,
}

async fn get_sise_handler(Query(params): Query<GetSiseParams>) -> impl IntoResponse {
    let timeframe = params.timeframe.unwrap_or_else(|| "day".to_string());

    match get_sise(
        &params.symbol,
        &params.start_time,
        &params.end_time,
        &timeframe,
    )
    .await
    {
        Ok(data) => (StatusCode::OK, data.into_response()),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Error fetching data".into_response(),
        ),
    }
}

async fn get_sise(
    symbol: &str,
    start_time: &str,
    end_time: &str,
    timeframe: &str,
) -> Result<String, reqwest::Error> {
    let client = Client::new();
    let mut params = HashMap::new();
    params.insert("symbol", symbol);
    params.insert("requestType", "1");
    params.insert("startTime", start_time);
    params.insert("endTime", end_time);
    params.insert("timeframe", timeframe);

    let url =
        Url::parse_with_params("https://api.finance.naver.com/siseJson.naver", &params).unwrap();

    let response = client.get(url).send().await?;
    
    let text = response.text().await?;
    let trimmed_text = text.trim().to_string();
   //  let json_data: Value = serde_json::from_str(&trimmed_text).unwrap();
   // let s= SerReader::new(&json_data);
   //  let df = JsonReader::new(&json_data.into())
   //      .infer_schema_len(None)
   //      .finish()
   //      .unwrap();
   println!("{:?}",trimmed_text);
    // Here you might need to process the response further to match the expected output
    Ok(trimmed_text)
}

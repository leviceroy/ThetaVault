/// Financial data fetcher — earnings dates (Nasdaq) + VIX (CNBC).
/// Yahoo Finance is no longer used due to aggressive rate-limiting (HTTP 429).
/// All functions are async and fail silently on network/parse errors.
use std::collections::HashMap;
use chrono::NaiveDate;

const USER_AGENT: &str =
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) \
     AppleWebKit/537.36 (KHTML, like Gecko) \
     Chrome/124.0.0.0 Safari/537.36";

fn build_client() -> Option<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(std::time::Duration::from_secs(8))
        .build()
        .ok()
}

/// Fetch next earnings dates for a list of tickers using Nasdaq's earnings calendar.
/// Queries the next 45 days in parallel and returns a map of ticker → NaiveDate.
pub async fn fetch_earnings_dates(tickers: &[String]) -> HashMap<String, NaiveDate> {
    if tickers.is_empty() {
        return HashMap::new();
    }

    let client = match build_client() {
        Some(c) => c,
        None => return HashMap::new(),
    };

    let today = chrono::Utc::now().date_naive();
    let ticker_set: std::collections::HashSet<String> = tickers.iter().cloned().collect();

    // Query 45 days of Nasdaq earnings calendar concurrently
    let dates: Vec<NaiveDate> = (0i64..=45)
        .map(|i| today + chrono::Duration::days(i))
        .collect();

    let mut handles = Vec::new();
    for &date in &dates {
        let c = client.clone();
        handles.push(tokio::spawn(async move {
            fetch_nasdaq_earnings_for_date(c, date).await
        }));
    }

    let mut result: HashMap<String, NaiveDate> = HashMap::new();
    for (date, handle) in dates.into_iter().zip(handles.into_iter()) {
        if let Ok(day_tickers) = handle.await {
            for ticker in day_tickers {
                if ticker_set.contains(&ticker) && !result.contains_key(&ticker) {
                    result.insert(ticker, date);
                }
            }
        }
    }
    result
}

/// Fetch current SPY price via CNBC (same pattern as fetch_vix).
pub async fn fetch_spy_price() -> Option<f64> {
    let client = build_client()?;
    let url = "https://quote.cnbc.com/quote-html-webservice/quote.htm\
               ?symbols=SPY&noform=1&output=json";
    let resp = client.get(url).send().await.ok()?;
    let json: serde_json::Value = resp.json().await.ok()?;
    json.pointer("/QuickQuoteResult/QuickQuote/0/last")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<f64>().ok())
}

/// Fetch beta vs SPY for each ticker from Yahoo Finance quoteSummary.
/// Any ticker that fails silently defaults to 1.0 (market-correlated).
pub async fn fetch_betas(tickers: &[String]) -> HashMap<String, f64> {
    if tickers.is_empty() { return HashMap::new(); }
    let client = match build_client() { Some(c) => c, None => return HashMap::new() };

    let mut handles = Vec::new();
    for ticker in tickers {
        let c = client.clone();
        let t = ticker.clone();
        handles.push(tokio::spawn(async move { fetch_beta_single(c, t).await }));
    }

    let mut result = HashMap::new();
    for (ticker, handle) in tickers.iter().zip(handles) {
        let beta = handle.await.ok().flatten().unwrap_or(1.0);
        result.insert(ticker.clone(), beta);
    }
    result
}

async fn fetch_beta_single(client: reqwest::Client, ticker: String) -> Option<f64> {
    let url = format!(
        "https://query2.finance.yahoo.com/v10/finance/quoteSummary/\
         {}?modules=defaultKeyStatistics",
        ticker
    );
    let resp = client
        .get(&url)
        .header("Accept", "application/json")
        .send().await.ok()?;
    let json: serde_json::Value = resp.json().await.ok()?;
    json.pointer("/quoteSummary/result/0/defaultKeyStatistics/beta/raw")
        .and_then(|v| v.as_f64())
}

/// Fetch current VIX price from CNBC's public quote API.
pub async fn fetch_vix() -> Option<f64> {
    let client = build_client()?;
    let url =
        "https://quote.cnbc.com/quote-html-webservice/quote.htm?symbols=.VIX&noform=1&output=json";

    let resp = client.get(url).send().await.ok()?;
    let json: serde_json::Value = resp.json().await.ok()?;

    // Path: /QuickQuoteResult/QuickQuote/0/last
    json.pointer("/QuickQuoteResult/QuickQuote/0/last")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<f64>().ok())
}

/// Fetch current prices for a list of tickers using CNBC (same pattern as fetch_spy_price).
pub async fn fetch_underlying_prices(tickers: &[String]) -> HashMap<String, f64> {
    if tickers.is_empty() { return HashMap::new(); }
    let client = match build_client() { Some(c) => c, None => return HashMap::new() };

    let mut handles = Vec::new();
    for ticker in tickers {
        let c = client.clone();
        let t = ticker.clone();
        handles.push(tokio::spawn(async move { fetch_price_single(c, t).await }));
    }

    let mut result = HashMap::new();
    for (ticker, handle) in tickers.iter().zip(handles) {
        if let Some(price) = handle.await.ok().flatten() {
            result.insert(ticker.clone(), price);
        }
    }
    result
}

async fn fetch_price_single(client: reqwest::Client, ticker: String) -> Option<f64> {
    let url = format!(
        "https://quote.cnbc.com/quote-html-webservice/quote.htm\
         ?symbols={}&noform=1&output=json",
        ticker
    );
    let resp = client.get(&url).send().await.ok()?;
    let json: serde_json::Value = resp.json().await.ok()?;
    json.pointer("/QuickQuoteResult/QuickQuote/0/last")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<f64>().ok())
}

/// Fetch all tickers reporting on a given date from Nasdaq's earnings calendar.
async fn fetch_nasdaq_earnings_for_date(client: reqwest::Client, date: NaiveDate) -> Vec<String> {
    let url = format!(
        "https://api.nasdaq.com/api/calendar/earnings?date={}",
        date.format("%Y-%m-%d")
    );

    let resp = client
        .get(&url)
        .header("Accept", "application/json, text/plain, */*")
        .header("Accept-Language", "en-US,en;q=0.9")
        .send()
        .await
        .ok();

    let json: serde_json::Value = match resp {
        Some(r) => r.json().await.unwrap_or_default(),
        None => return vec![],
    };

    json.pointer("/data/rows")
        .and_then(|v| v.as_array())
        .map(|rows| {
            rows.iter()
                .filter_map(|row| {
                    row.get("symbol")
                        .and_then(|s| s.as_str())
                        .map(String::from)
                })
                .collect()
        })
        .unwrap_or_default()
}

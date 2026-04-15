/// Financial data fetcher — earnings dates (Nasdaq) + VIX (CNBC).
/// Yahoo Finance is no longer used due to aggressive rate-limiting (HTTP 429).
/// All functions are async and fail silently on network/parse errors.
///
/// Exception: fetch_spy_monthly_returns uses Yahoo Finance v8 chart API (same
/// as fetch_betas) with a Mozilla User-Agent to retrieve historical monthly data.
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

/// Fetch SPY monthly returns (open→close %) from Yahoo Finance v8 chart API.
/// Returns a map of (year, month) → return_pct.  Fails silently → empty map.
pub async fn fetch_spy_monthly_returns(
    start_year: i32, start_month: u32,
) -> HashMap<(i32, u32), f64> {
    use chrono::Datelike;
    let client = match build_client() { Some(c) => c, None => return HashMap::new() };
    let period1 = chrono::NaiveDate::from_ymd_opt(start_year, start_month, 1)
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .map(|dt| chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(dt, chrono::Utc).timestamp())
        .unwrap_or(0);
    let period2 = chrono::Utc::now().timestamp() + 86_400;
    let url = format!(
        "https://query1.finance.yahoo.com/v8/finance/chart/SPY\
         ?interval=1mo&period1={period1}&period2={period2}&includePrePost=false"
    );
    let resp = match client.get(&url)
        .header("Accept", "application/json")
        .send().await { Ok(r) => r, Err(_) => return HashMap::new() };
    let json: serde_json::Value = match resp.json().await { Ok(j) => j, Err(_) => return HashMap::new() };

    let result = match json.pointer("/chart/result/0") { Some(r) => r, None => return HashMap::new() };
    let timestamps = match result["timestamp"].as_array()  { Some(t) => t, None => return HashMap::new() };
    let opens  = match result.pointer("/indicators/quote/0/open") .and_then(|v| v.as_array()) { Some(a) => a, None => return HashMap::new() };
    let closes = match result.pointer("/indicators/quote/0/close").and_then(|v| v.as_array()) { Some(a) => a, None => return HashMap::new() };

    let mut map = HashMap::new();
    for (i, ts) in timestamps.iter().enumerate() {
        if let (Some(ts_i), Some(open), Some(close)) = (
            ts.as_i64(),
            opens.get(i).and_then(|v| v.as_f64()),
            closes.get(i).and_then(|v| v.as_f64()),
        ) {
            if open > 0.0 {
                let dt = chrono::DateTime::from_timestamp(ts_i, 0)
                    .unwrap_or_default().naive_utc();
                map.insert((dt.year(), dt.month()), (close / open - 1.0) * 100.0);
            }
        }
    }
    map
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

/// Fetch current SPX (S&P 500 index) price via CNBC.
pub async fn fetch_spx_price() -> Option<f64> {
    let client = build_client()?;
    let url = "https://quote.cnbc.com/quote-html-webservice/quote.htm\
               ?symbols=.SPX&noform=1&output=json";
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
/// Sends browser-like headers (Accept + Referer) to avoid Akamai bot detection,
/// which can block automated requests to index symbols like .VIX.
pub async fn fetch_vix() -> Option<f64> {
    let client = build_client()?;
    let url =
        "https://quote.cnbc.com/quote-html-webservice/quote.htm?symbols=.VIX&noform=1&output=json";

    let resp = client
        .get(url)
        .header("Accept", "application/json, text/javascript, */*; q=0.01")
        .header("Referer", "https://www.cnbc.com/quotes/.VIX")
        .send().await.ok()?;
    let json: serde_json::Value = resp.json().await.ok()?;

    // Path: /QuickQuoteResult/QuickQuote/0/last
    json.pointer("/QuickQuoteResult/QuickQuote/0/last")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<f64>().ok())
}

/// Map derivative tickers (weekly index options) to the underlying that CNBC recognises.
/// SPXW and XSP are S&P weekly option tickers; the underlying is SPX.
/// NDXP is the Nasdaq weekly; NDX is the underlying.
fn price_alias(ticker: &str) -> &str {
    match ticker {
        "SPXW" | "XSP"  => "SPX",
        "NDXP"           => "NDX",
        "RUTW"           => "RUT",
        other            => other,
    }
}

/// Fetch current prices for a list of tickers using CNBC (same pattern as fetch_spy_price).
pub async fn fetch_underlying_prices(tickers: &[String]) -> HashMap<String, f64> {
    if tickers.is_empty() { return HashMap::new(); }
    let client = match build_client() { Some(c) => c, None => return HashMap::new() };

    let mut handles = Vec::new();
    for ticker in tickers {
        let c = client.clone();
        // Fetch the canonical underlying symbol but store back under the original ticker name.
        let fetch_as = price_alias(ticker).to_string();
        handles.push(tokio::spawn(async move { fetch_price_single(c, fetch_as).await }));
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

/// Fetch current ATM implied volatility for each ticker using Yahoo Finance v7 options chain.
/// Looks at the first (nearest) expiry, finds the strike closest to spot, averages call+put IV.
/// Returns a map of ticker → IV (as a fraction, e.g. 0.30 = 30%). Fails silently per ticker.
pub async fn fetch_atm_ivs(tickers: &[String]) -> HashMap<String, f64> {
    if tickers.is_empty() { return HashMap::new(); }
    let client = match build_client() { Some(c) => c, None => return HashMap::new() };

    let mut handles = Vec::new();
    for ticker in tickers {
        let c = client.clone();
        let t = ticker.clone();
        handles.push(tokio::spawn(async move { fetch_atm_iv_single(c, t).await }));
    }

    let mut result = HashMap::new();
    for (ticker, handle) in tickers.iter().zip(handles) {
        if let Some(iv) = handle.await.ok().flatten() {
            if iv > 0.0 {
                result.insert(ticker.clone(), iv);
            }
        }
    }
    result
}

async fn fetch_atm_iv_single(client: reqwest::Client, ticker: String) -> Option<f64> {
    let base_url = format!(
        "https://query2.finance.yahoo.com/v7/finance/options/{}",
        ticker
    );
    let resp = client
        .get(&base_url)
        .header("Accept", "application/json")
        .send().await.ok()?;
    let json: serde_json::Value = resp.json().await.ok()?;

    let result = json.pointer("/optionChain/result/0")?;
    let spot = result.pointer("/quote/regularMarketPrice")
        .and_then(|v| v.as_f64())
        .filter(|&p| p > 0.0)?;

    // Pick nearest expiry ≥ 14 DTE to avoid weekly-pin IV distortions.
    // Yahoo returns expirationDates as Unix timestamps; the options array matches by index.
    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).ok()?.as_secs() as i64;
    let min_secs = now_secs + 14 * 86400;

    let exp_dates: Option<Vec<i64>> = result.pointer("/expirationDates")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_i64()).collect());

    // Find the index of the first expiry >= 14 days out
    let preferred_exp: Option<i64> = exp_dates.as_ref().and_then(|dates| {
        dates.iter().copied().find(|&ts| ts >= min_secs)
    });

    // If the preferred expiry is not the first one, re-fetch with ?date= param
    let (json2, result2_owner);
    let result = if let Some(exp_ts) = preferred_exp.filter(|_| {
        // Only re-fetch if first expiry is too short (<14d)
        exp_dates.as_ref().and_then(|d| d.first().copied()).map_or(false, |first| first < min_secs)
    }) {
        let url2 = format!("{}?date={}", base_url, exp_ts);
        let resp2 = client.get(&url2).header("Accept", "application/json").send().await.ok()?;
        json2 = resp2.json::<serde_json::Value>().await.ok()?;
        result2_owner = json2.pointer("/optionChain/result/0")?;
        result2_owner
    } else {
        result
    };

    let options = result.pointer("/options/0")?;

    // Collect all strikes with (call_iv, put_iv) from first expiry
    let calls = options["calls"].as_array()?;
    let puts  = options["puts"].as_array()?;

    // Build put IV map by strike for quick lookup
    let put_iv: HashMap<u64, f64> = puts.iter().filter_map(|p| {
        let k = p["strike"].as_f64()?;
        let iv = p["impliedVolatility"].as_f64()?;
        Some(((k * 100.0) as u64, iv))
    }).collect();

    // Find call with strike closest to spot; use paired put IV
    let best = calls.iter()
        .filter_map(|c| {
            let k  = c["strike"].as_f64()?;
            let iv = c["impliedVolatility"].as_f64()?;
            Some((k, iv))
        })
        .min_by(|(ka, _), (kb, _)| {
            (ka - spot).abs().partial_cmp(&(kb - spot).abs()).unwrap_or(std::cmp::Ordering::Equal)
        })?;

    let call_iv = best.1;
    let put_iv_val = put_iv.get(&((best.0 * 100.0) as u64)).copied().unwrap_or(call_iv);
    Some((call_iv + put_iv_val) / 2.0)
}

/// Fetch GICS sector for a list of tickers via Yahoo Finance search API.
/// Uses /v1/finance/search (less rate-limited than quoteSummary). Fails silently per ticker.
pub async fn fetch_sectors(tickers: &[String]) -> HashMap<String, String> {
    if tickers.is_empty() { return HashMap::new(); }
    let client = match build_client() { Some(c) => c, None => return HashMap::new() };

    let mut handles = Vec::new();
    for ticker in tickers {
        let c = client.clone();
        let t = ticker.clone();
        handles.push(tokio::spawn(async move { fetch_sector_single(c, t).await }));
    }

    let mut result = HashMap::new();
    for (ticker, handle) in tickers.iter().zip(handles) {
        if let Some(sector) = handle.await.ok().flatten() {
            result.insert(ticker.clone(), sector);
        }
    }
    result
}

/// Static sector map for ETFs that Yahoo Finance does not classify under GICS sectors.
fn etf_sector_fallback(ticker: &str) -> Option<&'static str> {
    match ticker {
        // Energy
        "USO" | "UNG" | "BOIL" | "KOLD" | "UCO" | "SCO" | "XLE" | "OIH" | "XOP" => Some("Energy"),
        // Materials / Commodities
        "GLD" | "IAU" | "GLDM" | "SLV" | "SIVR" | "GDX" | "GDXJ" | "COPX" | "SLX"
        | "PDBC" | "DJP" | "GSG" | "XLB" => Some("Materials"),
        // Technology
        "QQQ" | "TQQQ" | "SQQQ" | "SMH" | "SOXX" | "XLK" | "ARKK" | "ARKG" | "ARKW"
        | "WCLD" | "IGV" => Some("Technology"),
        // Financials
        "XLF" | "KRE" | "KBE" | "IAI" | "IAK" => Some("Financials"),
        // Health Care
        "XLV" | "XBI" | "IBB" | "LABU" | "LABD" => Some("Health Care"),
        // Consumer Discretionary
        "XLY" | "XIRT" => Some("Consumer Discretionary"),
        // Consumer Staples
        "XLP" => Some("Consumer Staples"),
        // Industrials
        "XLI" | "ITA" | "XAR" => Some("Industrials"),
        // Utilities
        "XLU" => Some("Utilities"),
        // Communication Services
        "XLC" => Some("Communication Services"),
        // Real Estate
        "XLRE" | "VNQ" | "IYR" => Some("Real Estate"),
        _ => None,
    }
}

async fn fetch_sector_single(client: reqwest::Client, ticker: String) -> Option<String> {
    // Use the search API — less rate-limited than quoteSummary, returns sector directly
    let url = format!(
        "https://query1.finance.yahoo.com/v1/finance/search\
         ?q={}&quotesCount=1&newsCount=0&enableFuzzyQuery=false",
        ticker
    );
    let resp = client
        .get(&url)
        .header("Accept", "application/json")
        .send().await.ok()?;
    let json: serde_json::Value = resp.json().await.ok()?;
    // Find the exact matching quote by symbol to avoid fuzzy-match wrong tickers
    let api_result = json.pointer("/quotes")
        .and_then(|v| v.as_array())
        .and_then(|quotes| {
            quotes.iter().find(|q| {
                q.get("symbol")
                    .and_then(|s| s.as_str())
                    .map_or(false, |s| s == ticker)
            })
        })
        .and_then(|q| q.get("sector"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // ETFs return no sector from the search API — fall back to static map
    api_result.or_else(|| etf_sector_fallback(&ticker).map(|s| s.to_string()))
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

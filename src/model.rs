use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct ApiResponse {
    pub status: String,
    pub data: Vec<OptionData>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct OptionData {
    #[allow(dead_code)]
    pub expiry: String,
    pub strike_price: f64,
    pub underlying_key: String,
    pub underlying_spot_price: f64,
    pub call_options: Option<OptionContract>,
    pub put_options: Option<OptionContract>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct OptionContract {
    pub instrument_key: String,
    pub market_data: MarketData,
    pub option_greeks: Option<OptionGreeks>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct OptionGreeks {
    #[serde(default)]
    pub delta: f64,
    #[serde(default)]
    pub iv: f64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MarketData {
    #[serde(default)]
    pub ltp: f64,
    #[serde(default)]
    pub oi: f64,
    #[serde(default)]
    pub bid_price: f64,
    #[serde(default)]
    pub ask_price: f64,
}

// --- Market Quote API Models ---

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct MarketQuoteResponse {
    pub status: String,
    pub data: HashMap<String, QuoteData>,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct QuoteData {
    pub instrument_token: String,
    pub symbol: String,
    pub last_price: f64,
    pub depth: Depth,
    pub ohlc: Ohlc,
    #[serde(default)]
    pub volume: i64,
    #[serde(default)]
    pub oi: f64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Depth {
    pub buy: Vec<DepthEntry>,
    pub sell: Vec<DepthEntry>,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct DepthEntry {
    pub quantity: i32,
    pub price: f64,
    pub orders: i32,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct Ohlc {
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
}

impl ApiResponse {
    pub fn generate_dummy_data() -> Vec<OptionData> {
        let json_data = include_str!("demo_data.json");
        let response: ApiResponse = serde_json::from_str(json_data).expect("Failed to parse demo data");
        response.data
    }
}

// Helper for dummy depth
impl QuoteData {
    pub fn dummy() -> Self {
        QuoteData {
            instrument_token: "DUMMY".to_string(),
            symbol: "DUMMY".to_string(),
            last_price: 100.0,
            depth: Depth {
                buy: vec![
                    DepthEntry { quantity: 100, price: 99.5, orders: 5 },
                    DepthEntry { quantity: 200, price: 99.0, orders: 3 },
                    DepthEntry { quantity: 150, price: 98.5, orders: 2 },
                    DepthEntry { quantity: 50, price: 98.0, orders: 1 },
                    DepthEntry { quantity: 20, price: 97.5, orders: 1 },
                ],
                sell: vec![
                    DepthEntry { quantity: 80, price: 100.5, orders: 4 },
                    DepthEntry { quantity: 120, price: 101.0, orders: 6 },
                    DepthEntry { quantity: 90, price: 101.5, orders: 2 },
                    DepthEntry { quantity: 40, price: 102.0, orders: 1 },
                    DepthEntry { quantity: 10, price: 102.5, orders: 1 },
                ]
            },
            ohlc: Ohlc { open: 100.0, high: 102.0, low: 98.0, close: 100.0 },
            volume: 1000,
            oi: 50000.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_sample_response() {
        // ... (Existing test slightly modified if needed to match new struct)
        // For brevity, keeping existing test logic conceptual or assuming it still compiles with minimal changes
        // since we mostly added fields.
        let json_data = r#"
        {
          "status": "success",
          "data": [
            {
              "expiry": "2025-02-13",
              "pcr": 7515.3,
              "strike_price": 21100,
              "underlying_key": "NSE_INDEX|Nifty 50",
              "underlying_spot_price": 22976.2,
              "call_options": {
                "instrument_key": "NSE_FO|51059",
                "market_data": {
                  "ltp": 2449.9,
                  "volume": 0,
                  "oi": 750,
                  "close_price": 2449.9,
                  "bid_price": 1856.65,
                  "bid_qty": 1125,
                  "ask_price": 1941.65,
                  "ask_qty": 1125,
                  "prev_oi": 1500
                },
                "option_greeks": {
                  "vega": 4.1731,
                  "theta": -472.8941,
                  "gamma": 0.0001,
                  "delta": 0.743,
                  "iv": 262.31,
                  "pop": 40.56
                }
              },
              "put_options": {
                "instrument_key": "NSE_FO|51060",
                "market_data": {
                  "ltp": 0.3,
                  "volume": 22315725,
                  "oi": 5636475,
                  "close_price": 0.35,
                  "bid_price": 0.3,
                  "bid_qty": 1979400,
                  "ask_price": 0.35,
                  "ask_qty": 2152500,
                  "prev_oi": 5797500
                },
                "option_greeks": {
                  "vega": 0.0568,
                  "theta": -1.2461,
                  "gamma": 0,
                  "delta": -0.0013,
                  "iv": 50.78,
                  "pop": 0.15
                }
              }
            }
          ]
        }
        "#;
        let response: ApiResponse = serde_json::from_str(json_data).expect("Failed to deserialize");
        assert_eq!(response.data[0].call_options.as_ref().unwrap().instrument_key, "NSE_FO|51059");
    }
}

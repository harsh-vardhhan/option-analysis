use serde::Deserialize;

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
    pub market_data: MarketData,
    pub option_greeks: Option<OptionGreeks>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct OptionGreeks {
    #[serde(default)]
    pub delta: f64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MarketData {
    pub ltp: f64,
    #[serde(default)]
    pub oi: f64,
    #[serde(default)]
    pub bid_price: f64,
    #[serde(default)]
    pub ask_price: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_sample_response() {
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
        assert_eq!(response.status, "success");
        assert_eq!(response.data.len(), 1);
        assert_eq!(response.data[0].expiry, "2025-02-13");
        assert_eq!(response.data[0].strike_price, 21100.0);
        
        let call = response.data[0].call_options.as_ref().unwrap();
        assert_eq!(call.market_data.ltp, 2449.9);

        let put = response.data[0].put_options.as_ref().unwrap();
        assert_eq!(put.market_data.ltp, 0.3);
    }
}

use crate::model::OptionData;
use crate::strategy::{Position, OptionType};
use std::cmp::Ordering;

pub struct StrategyBuilder;

impl StrategyBuilder {
    pub fn build(strategy_name: &str, data: &[OptionData]) -> Result<(Vec<Position>, String), String> {
        let mut positions = Vec::new();
        let message;

        if strategy_name == "Call Credit Spread" {
            // Sell Call (Delta ~ 0.3) + Buy Call (Strike + 100)
            let sell_leg_idx = Self::find_call_with_filter(data, 0.3, |spread, delta| spread <= 20.0 && delta > 0.1 && delta < 0.9);
            
            if let Some(idx) = sell_leg_idx {
                let sell_item = &data[idx];
                let sell_strike = sell_item.strike_price;
                let sell_price = sell_item.call_options.as_ref()
                    .ok_or_else(|| format!("Call data missing for strike {}", sell_strike))?
                    .market_data.ltp;
                
                let buy_strike_target = sell_strike + 100.0;
                let buy_leg = data.iter().find(|d| (d.strike_price - buy_strike_target).abs() < 1.0);
                
                if let Some(buy_item) = buy_leg {
                    let buy_price = buy_item.call_options.as_ref().map(|o| o.market_data.ltp).unwrap_or(0.0); 
                    positions.push(Position { strike: sell_strike, kind: OptionType::Call, qty: -1, entry_price: sell_price });
                    positions.push(Position { strike: buy_strike_target, kind: OptionType::Call, qty: 1, entry_price: buy_price });
                    message = format!("Applied: CCS ({}/{})", sell_strike, buy_strike_target);
                } else { return Err(String::from("Error: Buy leg (+100) not found")); }
            } else { return Err(String::from("Error: No Sell leg (Delta~0.3, Spread<=20) found")); }
            
        } else if strategy_name == "Put Credit Spread" {
            // Sell Put (Delta ~ -0.3) + Buy Put (Strike - 100)
            let sell_leg_idx = Self::find_put_with_filter(data, -0.3, |spread, delta| spread <= 20.0 && delta > -0.9 && delta < -0.1);

            if let Some(idx) = sell_leg_idx {
                let sell_item = &data[idx];
                let sell_strike = sell_item.strike_price;
                let sell_price = sell_item.put_options.as_ref()
                    .ok_or_else(|| format!("Put data missing for strike {}", sell_strike))?
                    .market_data.ltp;
                
                let buy_strike_target = sell_strike - 100.0;
                let buy_leg = data.iter().find(|d| (d.strike_price - buy_strike_target).abs() < 1.0);
                
                if let Some(buy_item) = buy_leg {
                    let buy_price = buy_item.put_options.as_ref().map(|o| o.market_data.ltp).unwrap_or(0.0); 
                    positions.push(Position { strike: sell_strike, kind: OptionType::Put, qty: -1, entry_price: sell_price });
                    positions.push(Position { strike: buy_strike_target, kind: OptionType::Put, qty: 1, entry_price: buy_price });
                    message = format!("Applied: PCS ({}/{})", sell_strike, buy_strike_target);
                } else { return Err(String::from("Error: Buy leg (-100) not found")); }
            } else { return Err(String::from("Error: No Sell leg (Delta~-0.3, Spread<=20) found")); }

        } else if strategy_name == "Call Debit Spread" {
            // Buy Call (Delta ~ 0.5) + Sell Call (Strike + 100)
            let buy_leg_idx = Self::find_call_with_filter(data, 0.5, |spread, delta| spread <= 20.0 && delta > 0.1 && delta < 0.9);

            if let Some(idx) = buy_leg_idx {
                let buy_item = &data[idx];
                let buy_strike = buy_item.strike_price;
                let buy_price = buy_item.call_options.as_ref()
                    .ok_or_else(|| format!("Call data missing for strike {}", buy_strike))?
                    .market_data.ltp;
                
                let sell_strike_target = buy_strike + 100.0;
                let sell_leg = data.iter().find(|d| (d.strike_price - sell_strike_target).abs() < 1.0);
                
                if let Some(sell_item) = sell_leg {
                    let sell_price = sell_item.call_options.as_ref().map(|o| o.market_data.ltp).unwrap_or(0.0); 
                    positions.push(Position { strike: buy_strike, kind: OptionType::Call, qty: 1, entry_price: buy_price });
                    positions.push(Position { strike: sell_strike_target, kind: OptionType::Call, qty: -1, entry_price: sell_price });
                    message = format!("Applied: CDS ({}/{})", buy_strike, sell_strike_target);
                } else { return Err(String::from("Error: Sell leg (+100) not found")); }
            } else { return Err(String::from("Error: No Buy leg (Delta~0.5) found")); }

        } else if strategy_name == "Put Debit Spread" {
            // Buy Put (Delta ~ -0.5) + Sell Put (Strike - 100)
            let buy_leg_idx = Self::find_put_with_filter(data, -0.5, |spread, delta| spread <= 20.0 && delta > -0.9 && delta < -0.1);

            if let Some(idx) = buy_leg_idx {
                let buy_item = &data[idx];
                let buy_strike = buy_item.strike_price;
                let buy_price = buy_item.put_options.as_ref()
                    .ok_or_else(|| format!("Put data missing for strike {}", buy_strike))?
                    .market_data.ltp;
                
                let sell_strike_target = buy_strike - 100.0;
                let sell_leg = data.iter().find(|d| (d.strike_price - sell_strike_target).abs() < 1.0);
                
                if let Some(sell_item) = sell_leg {
                    let sell_price = sell_item.put_options.as_ref().map(|o| o.market_data.ltp).unwrap_or(0.0); 
                    positions.push(Position { strike: buy_strike, kind: OptionType::Put, qty: 1, entry_price: buy_price });
                    positions.push(Position { strike: sell_strike_target, kind: OptionType::Put, qty: -1, entry_price: sell_price });
                    message = format!("Applied: PDS ({}/{})", buy_strike, sell_strike_target);
                } else { return Err(String::from("Error: Sell leg (-100) not found")); }
            } else { return Err(String::from("Error: No Buy leg (Delta~-0.5) found")); }
        
        } else if strategy_name == "Long Call" {
             if let Some(idx) = Self::find_call_by_delta(data, 0.5) {
                 let item = &data[idx];
                 let price = item.call_options.as_ref()
                    .ok_or_else(|| format!("Call data missing for strike {}", item.strike_price))?
                    .market_data.ltp;
                 positions.push(Position { strike: item.strike_price, kind: OptionType::Call, qty: 1, entry_price: price });
                 message = format!("Applied: Long Call ({})", item.strike_price);
             } else { return Err(String::from("Error: Delta ~0.5 not found")); }

        } else if strategy_name == "Long Put" {
             if let Some(idx) = Self::find_put_by_delta(data, -0.5) {
                 let item = &data[idx];
                 let price = item.put_options.as_ref()
                    .ok_or_else(|| format!("Put data missing for strike {}", item.strike_price))?
                    .market_data.ltp;
                 positions.push(Position { strike: item.strike_price, kind: OptionType::Put, qty: 1, entry_price: price });
                 message = format!("Applied: Long Put ({})", item.strike_price);
             } else { return Err(String::from("Error: Delta ~-0.5 not found")); }

        } else if strategy_name == "Short Call" {
             if let Some(idx) = Self::find_call_by_delta(data, 0.3) {
                 let item = &data[idx];
                 let price = item.call_options.as_ref()
                    .ok_or_else(|| format!("Call data missing for strike {}", item.strike_price))?
                    .market_data.ltp;
                 positions.push(Position { strike: item.strike_price, kind: OptionType::Call, qty: -1, entry_price: price });
                 message = format!("Applied: Short Call ({})", item.strike_price);
             } else { return Err(String::from("Error: Delta ~0.3 not found")); }

        } else if strategy_name == "Short Put" {
             if let Some(idx) = Self::find_put_by_delta(data, -0.3) {
                 let item = &data[idx];
                 let price = item.put_options.as_ref()
                    .ok_or_else(|| format!("Put data missing for strike {}", item.strike_price))?
                    .market_data.ltp;
                 positions.push(Position { strike: item.strike_price, kind: OptionType::Put, qty: -1, entry_price: price });
                 message = format!("Applied: Short Put ({})", item.strike_price);
             } else { return Err(String::from("Error: Delta ~-0.3 not found")); }

        } else if strategy_name == "Short Straddle" {
             let call_idx = Self::find_call_by_delta(data, 0.5);
             let put_idx = Self::find_put_by_delta(data, -0.5);
             
             if let (Some(c_idx), Some(_)) = (call_idx, put_idx) {
                 let strike = data[c_idx].strike_price;
                 let call_price = data[c_idx].call_options.as_ref()
                    .ok_or_else(|| format!("Call data missing for strike {}", strike))?
                    .market_data.ltp;
                 
                 if let Some(put_item) = data.iter().find(|d| (d.strike_price - strike).abs() < 1.0) {
                      let put_price = put_item.put_options.as_ref()
                        .ok_or_else(|| format!("Put data missing for strike {}", put_item.strike_price))?
                        .market_data.ltp;
                      
                      positions.push(Position { strike, kind: OptionType::Call, qty: -1, entry_price: call_price });
                      positions.push(Position { strike, kind: OptionType::Put, qty: -1, entry_price: put_price });
                      message = format!("Applied: Short Straddle ({})", strike);
                 } else { return Err(String::from("Error: Put leg for Straddle not found")); }
             } else { return Err(String::from("Error: ATM Legs not found")); }

        } else if strategy_name == "Short Strangle" {
             let call_idx = Self::find_call_by_delta(data, 0.16);
             let put_idx = Self::find_put_by_delta(data, -0.16);
             
             if let (Some(c_idx), Some(p_idx)) = (call_idx, put_idx) {
                 let c_item = &data[c_idx];
                 let p_item = &data[p_idx];
                 
                 let call_price = c_item.call_options.as_ref()
                    .ok_or_else(|| format!("Call data missing for strike {}", c_item.strike_price))?
                    .market_data.ltp;
                 
                 let put_price = p_item.put_options.as_ref()
                    .ok_or_else(|| format!("Put data missing for strike {}", p_item.strike_price))?
                    .market_data.ltp;

                 positions.push(Position { strike: c_item.strike_price, kind: OptionType::Call, qty: -1, entry_price: call_price });
                 positions.push(Position { strike: p_item.strike_price, kind: OptionType::Put, qty: -1, entry_price: put_price });
                 message = format!("Applied: Short Strangle ({}/{})", p_item.strike_price, c_item.strike_price);
             } else { return Err(String::from("Error: OTM Legs (~0.16) not found")); }

        } else if strategy_name == "Iron Butterfly" {
             let center_idx = Self::find_call_by_delta(data, 0.5);
             if let Some(c_idx) = center_idx {
                 let center_strike = data[c_idx].strike_price;
                 let upper_strike = center_strike + 200.0;
                 let lower_strike = center_strike - 200.0;
                 
                 let center_item = data.iter().find(|d| (d.strike_price - center_strike).abs() < 1.0);
                 let upper_item = data.iter().find(|d| (d.strike_price - upper_strike).abs() < 1.0);
                 let lower_item = data.iter().find(|d| (d.strike_price - lower_strike).abs() < 1.0);
                 
                 if let (Some(c), Some(u), Some(l)) = (center_item, upper_item, lower_item) {
                     let c_call_price = c.call_options.as_ref()
                        .ok_or_else(|| format!("Call data missing for strike {}", c.strike_price))?
                        .market_data.ltp;
                     let c_put_price = c.put_options.as_ref()
                        .ok_or_else(|| format!("Put data missing for strike {}", c.strike_price))?
                        .market_data.ltp;
                     let u_call_price = u.call_options.as_ref()
                        .ok_or_else(|| format!("Call data missing for strike {}", u.strike_price))?
                        .market_data.ltp;
                     let l_put_price = l.put_options.as_ref()
                        .ok_or_else(|| format!("Put data missing for strike {}", l.strike_price))?
                        .market_data.ltp;

                     positions.push(Position { strike: center_strike, kind: OptionType::Call, qty: -1, entry_price: c_call_price });
                     positions.push(Position { strike: center_strike, kind: OptionType::Put, qty: -1, entry_price: c_put_price });
                     positions.push(Position { strike: upper_strike, kind: OptionType::Call, qty: 1, entry_price: u_call_price });
                     positions.push(Position { strike: lower_strike, kind: OptionType::Put, qty: 1, entry_price: l_put_price });
                     message = format!("Applied: Iron Fly ({})", center_strike);
                 } else { return Err(String::from("Error: Wings or Center not found")); }
             } else { return Err(String::from("Error: ATM Center not found")); }

        } else if strategy_name == "Iron Condor" {
             let call_idx = Self::find_call_by_delta(data, 0.16);
             let put_idx = Self::find_put_by_delta(data, -0.16);
             
             if let (Some(c_idx), Some(p_idx)) = (call_idx, put_idx) {
                 let short_call_strike = data[c_idx].strike_price;
                 let short_put_strike = data[p_idx].strike_price;
                 
                 let long_call_strike = short_call_strike + 200.0;
                 let long_put_strike = short_put_strike - 200.0;
                 
                 let sc_item = &data[c_idx];
                 let sp_item = &data[p_idx];
                 let lc_item = data.iter().find(|d| (d.strike_price - long_call_strike).abs() < 1.0);
                 let lp_item = data.iter().find(|d| (d.strike_price - long_put_strike).abs() < 1.0);
                 
                 if let (Some(lc), Some(lp)) = (lc_item, lp_item) {
                     let sc_price = sc_item.call_options.as_ref()
                        .ok_or_else(|| format!("Call data missing for strike {}", sc_item.strike_price))?
                        .market_data.ltp;
                     let sp_price = sp_item.put_options.as_ref()
                        .ok_or_else(|| format!("Put data missing for strike {}", sp_item.strike_price))?
                        .market_data.ltp;
                     let lc_price = lc.call_options.as_ref()
                        .ok_or_else(|| format!("Call data missing for strike {}", lc.strike_price))?
                        .market_data.ltp;
                     let lp_price = lp.put_options.as_ref()
                        .ok_or_else(|| format!("Put data missing for strike {}", lp.strike_price))?
                        .market_data.ltp;

                     positions.push(Position { strike: short_call_strike, kind: OptionType::Call, qty: -1, entry_price: sc_price });
                     positions.push(Position { strike: short_put_strike, kind: OptionType::Put, qty: -1, entry_price: sp_price });
                     positions.push(Position { strike: long_call_strike, kind: OptionType::Call, qty: 1, entry_price: lc_price });
                     positions.push(Position { strike: long_put_strike, kind: OptionType::Put, qty: 1, entry_price: lp_price });
                     message = format!("Applied: Iron Condor ({}/{})", short_put_strike, short_call_strike);
                 } else { return Err(String::from("Error: Wings not found")); }
             } else { return Err(String::from("Error: Strangle legs not found")); }

        } else {
            return Err(format!("Unknown strategy: {}", strategy_name));
        }

        Ok((positions, message))
    }

    fn find_call_by_delta(data: &[OptionData], target: f64) -> Option<usize> {
        Self::find_call_with_filter(data, target, |_, _| true)
    }

    fn find_put_by_delta(data: &[OptionData], target: f64) -> Option<usize> {
         Self::find_put_with_filter(data, target, |_, _| true)
    }

    fn find_call_with_filter<F>(data: &[OptionData], target: f64, filter: F) -> Option<usize>
    where F: Fn(f64, f64) -> bool 
    {
        data.iter().enumerate()
            .filter(|(_, d)| {
                if let Some(call) = &d.call_options {
                    let spread = (call.market_data.ask_price - call.market_data.bid_price).abs();
                    let delta = call.option_greeks.as_ref().map(|g| g.delta).unwrap_or(0.0);
                    filter(spread, delta)
                } else { false }
            })
            .min_by(|(_, a), (_, b)| {
                let da = a.call_options.as_ref().and_then(|o| o.option_greeks.as_ref()).map(|g| g.delta).unwrap_or(0.0);
                let db = b.call_options.as_ref().and_then(|o| o.option_greeks.as_ref()).map(|g| g.delta).unwrap_or(0.0);
                (da - target).abs().partial_cmp(&(db - target).abs()).unwrap_or(Ordering::Equal)
            })
            .map(|(i, _)| i)
    }

    fn find_put_with_filter<F>(data: &[OptionData], target: f64, filter: F) -> Option<usize>
    where F: Fn(f64, f64) -> bool
    {
        data.iter().enumerate()
            .filter(|(_, d)| {
                if let Some(put) = &d.put_options {
                    let spread = (put.market_data.ask_price - put.market_data.bid_price).abs();
                    let delta = put.option_greeks.as_ref().map(|g| g.delta).unwrap_or(0.0);
                    filter(spread, delta)
                } else { false }
            })
            .min_by(|(_, a), (_, b)| {
                let da = a.put_options.as_ref().and_then(|o| o.option_greeks.as_ref()).map(|g| g.delta).unwrap_or(0.0);
                let db = b.put_options.as_ref().and_then(|o| o.option_greeks.as_ref()).map(|g| g.delta).unwrap_or(0.0);
                (da - target).abs().partial_cmp(&(db - target).abs()).unwrap_or(Ordering::Equal)
            })
            .map(|(i, _)| i)
    }
}

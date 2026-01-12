use anyhow::Result;
use std::time::Duration;
use tokio::sync::mpsc;

mod app;
mod model;
mod ui;
mod strategy;
mod strategy_builder;
mod portfolio;
mod tui;
#[cfg(test)]
mod app_tests;

use app::App;
use model::ApiResponse;
use tui::Tui;

// API Configuration
const UPSTOX_API_BASE: &str = "https://api.upstox.com/v2/option/chain";
const INSTRUMENT_KEY: &str = "NSE_INDEX|Nifty 50";

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Initialize TUI (Handling Terminal Setup/Teardown via RAII)
    let mut tui = Tui::new()?;

    // 2. Setup App
    let mut app = App::new();
    // Default to first available expiry, or a fallback if empty
    let initial_expiry = app.available_expiries.first().cloned().unwrap_or_else(|| String::from("29 Jan 2026"));
    
    // Create channel for expiry updates
    let (expiry_tx, expiry_rx) = tokio::sync::watch::channel(initial_expiry.clone());

    // 3. Get Access Token (TUI Mode)
    let validation_url = format!(
        "{}?instrument_key={}&expiry_date={}", 
        UPSTOX_API_BASE, 
        urlencoding::encode(INSTRUMENT_KEY), 
        initial_expiry
    );
    
    // Pass the terminal instance from our Tui wrapper
    let setup_result = ui::setup::run_setup_tui(&mut tui.terminal, &validation_url).await?;

    // 4. Setup Data Channel
    // We now use TuiMessage to support both options data and quotes
    let (tx, rx) = mpsc::channel(10);
    // Channel for TUI to request quotes (string instrument key)
    let (quote_request_tx, mut quote_request_rx) = mpsc::channel::<String>(50);

    // 5. Background Data Fetcher
    match setup_result {
        ui::setup::SetupResult::Token(token) => {
            let token_clone = token.clone();
            let mut expiry_rx_clone = expiry_rx.clone();
            let main_tx = tx.clone(); 
            
            // TASK A: Option Chain Polling
            tokio::spawn(async move {
                let client = reqwest::Client::new();
                
                loop {
                    // Get current expiry from watch channel
                    let current_expiry = expiry_rx_clone.borrow_and_update().clone();

                    let url = format!(
                        "{}?instrument_key={}&expiry_date={}", 
                        UPSTOX_API_BASE, 
                        urlencoding::encode(INSTRUMENT_KEY), 
                        current_expiry
                    );

                    let res = client
                        .get(&url)
                        .header("Content-Type", "application/json")
                        .header("Accept", "application/json")
                        .header("Authorization", format!("Bearer {}", token_clone))
                        .send()
                        .await;
        
                    match res {
                        Ok(response) => {
                            if let Ok(api_response) = response.json::<ApiResponse>().await {
                                if !api_response.data.is_empty() {
                                    let _ = main_tx.send(tui::TuiMessage::OptionChain(api_response.data)).await;
                                }
                            }
                        }
                        Err(e) => {
                             eprintln!("Error fetching data: {}", e);
                        }
                    }
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            });

            // TASK B: Quote Fetcher (Auto-Refresh)
            let token_quote = token.clone();
            let quote_result_tx = tx.clone();
            
            tokio::spawn(async move {
                let client = reqwest::Client::new();
                let mut current_key: Option<String> = None;
                // Periodic timer
                let mut interval = tokio::time::interval(Duration::from_secs(1));
                // Set missed tick behavior to skip, to avoid burst fetches
                interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

                loop {
                    tokio::select! {
                        // 1. Handle New Instrument Selection (High Priority)
                        res = quote_request_rx.recv() => {
                            match res {
                                Some(new_key) => {
                                    current_key = Some(new_key);
                                    // Fetch immediately on new selection
                                    if let Some(instr_key) = &current_key {
                                        let url = format!("https://api.upstox.com/v2/market-quote/quotes?instrument_key={}", instr_key);
                                        if let Ok(response) = client.get(&url)
                                            .header("Content-Type", "application/json")
                                            .header("Accept", "application/json")
                                            .header("Authorization", format!("Bearer {}", token_quote))
                                            .send().await 
                                        {
                                            if let Ok(quote_res) = response.json::<model::MarketQuoteResponse>().await {
                                                if let Some(data) = quote_res.data.values().next() {
                                                    let _ = quote_result_tx.send(tui::TuiMessage::Quote(data.clone())).await;
                                                }
                                            }
                                        }
                                    }
                                    // Reset timer so we don't double-fetch immediately
                                    interval.reset();
                                },
                                None => break, // Channel closed
                            }
                        }
                        // 2. Periodic Refresh
                        _ = interval.tick() => {
                             if let Some(instr_key) = &current_key {
                                let url = format!("https://api.upstox.com/v2/market-quote/quotes?instrument_key={}", instr_key);
                                if let Ok(response) = client.get(&url)
                                    .header("Content-Type", "application/json")
                                    .header("Accept", "application/json")
                                    .header("Authorization", format!("Bearer {}", token_quote))
                                    .send().await 
                                {
                                    if let Ok(quote_res) = response.json::<model::MarketQuoteResponse>().await {
                                        if let Some(data) = quote_res.data.values().next() {
                                            let _ = quote_result_tx.send(tui::TuiMessage::Quote(data.clone())).await;
                                        }
                                    }
                                }
                             }
                        }
                    }
                }
            });
        },
        ui::setup::SetupResult::Demo => {
            // Demo loops
            let demo_tx = tx.clone();
            tokio::spawn(async move {
                // Send initial data
                let dummy_data = ApiResponse::generate_dummy_data();
                let _ = demo_tx.send(tui::TuiMessage::OptionChain(dummy_data)).await;
                
                loop {
                    tokio::time::sleep(Duration::from_secs(2)).await;
                    let dummy_data = ApiResponse::generate_dummy_data();
                    let _ = demo_tx.send(tui::TuiMessage::OptionChain(dummy_data)).await;
                }
            });

            // Demo Quote (Auto-Refresh)
            let demo_quote_tx = tx.clone();
            tokio::spawn(async move {
                let mut current_key: Option<String> = None;
                let mut interval = tokio::time::interval(Duration::from_secs(1));
                
                loop {
                    tokio::select! {
                        res = quote_request_rx.recv() => {
                             match res {
                                Some(k) => {
                                    current_key = Some(k);
                                    // Immediate update
                                    let dummy_quote = model::QuoteData::dummy();
                                    let _ = demo_quote_tx.send(tui::TuiMessage::Quote(dummy_quote)).await;
                                    interval.reset();
                                },
                                None => break,
                             }
                        }
                        _ = interval.tick() => {
                            if current_key.is_some() {
                                let dummy_quote = model::QuoteData::dummy();
                                let _ = demo_quote_tx.send(tui::TuiMessage::Quote(dummy_quote)).await;
                            }
                        }
                    }
                }
            });
        }
    }

    // 6. Run the Event Loop
    tui.run(&mut app, rx, expiry_tx, quote_request_tx).await?;

    Ok(())
}

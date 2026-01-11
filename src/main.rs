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
    let (tx, rx) = mpsc::channel(10);

    // 5. Background Data Fetcher
    match setup_result {
        ui::setup::SetupResult::Token(token) => {
            let token_clone = token.clone();
            let mut expiry_rx_clone = expiry_rx.clone();
            
            tokio::spawn(async move {
                let client = reqwest::Client::new();
                
                loop {
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
                                    let _ = tx.send(api_response.data).await;
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
        },
        ui::setup::SetupResult::Demo => {
            tokio::spawn(async move {
                // Send initial data
                let dummy_data = ApiResponse::generate_dummy_data();
                let _ = tx.send(dummy_data).await;
                
                loop {
                    tokio::time::sleep(Duration::from_secs(2)).await;
                    let dummy_data = ApiResponse::generate_dummy_data();
                    let _ = tx.send(dummy_data).await;
                }
            });
        }
    }

    // 6. Run the Event Loop
    tui.run(&mut app, rx, expiry_tx).await?;

    Ok(())
}

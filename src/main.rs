use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io::{self}, time::Duration};
use tokio::sync::mpsc;

mod app;
mod model;
mod ui;
mod strategy;
mod strategy_builder;
mod portfolio;
#[cfg(test)]
mod app_tests;

use app::App;
use model::ApiResponse;

// API Configuration
const UPSTOX_API_BASE: &str = "https://api.upstox.com/v2/option/chain";
const INSTRUMENT_KEY: &str = "NSE_INDEX|Nifty 50";
// EXPIRY_DATE constant removed; now dynamic.

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Setup Terminal Early
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // 2. Setup App Early (to get valid expiry for validation URL)
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
    let setup_result = ui::setup::run_setup_tui(&mut terminal, &validation_url).await?;

    // 4. Setup Data Channel
    let (tx, mut rx) = mpsc::channel(10);

    // 5. Background Data Fetcher
    match setup_result {
        ui::setup::SetupResult::Token(token) => {
            let token_clone = token.clone();
            // Move receiver into background task
            let mut expiry_rx_clone = expiry_rx.clone();
            
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
                // Send initial data immediately
                let dummy_data = ApiResponse::generate_dummy_data();
                let _ = tx.send(dummy_data).await;
                
                // Simulate updates
                loop {
                    tokio::time::sleep(Duration::from_secs(2)).await;
                    // In a real demo we might jitter the prices here, but for now static is fine
                    // or re-generate to simulate slight noise if I added rand.
                     let dummy_data = ApiResponse::generate_dummy_data();
                    let _ = tx.send(dummy_data).await;
                }
            });
        }
    }

    // 6. Main Event Loop
    let tick_rate = Duration::from_millis(250);
    let mut last_tick = std::time::Instant::now();

    loop {
        terminal.draw(|f| ui::draw(f, &mut app))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == event::KeyEventKind::Press {
                    if app.show_help {
                         match key.code {
                            KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') => app.toggle_help(),
                            KeyCode::Char('s') | KeyCode::Char('S') if key.modifiers.contains(event::KeyModifiers::SHIFT) => app.toggle_help(),
                            _ => {}
                         }
                    } else if key.modifiers.contains(event::KeyModifiers::SHIFT) {
                        match key.code {
                            KeyCode::Down => app.move_position_row(1),
                            KeyCode::Up => app.move_position_row(-1),
                            KeyCode::Left => {
                                // Try moving position col first (original behavior)
                                // Only if it doesn't handle it? No, wait. 
                                // Original request: "switching to past expiries (should not work once it's the first expiry):shift + ←"
                                // Original Code: `KeyCode::Left | KeyCode::Right => app.move_position_col(),`
                                // Wait, the original code used Shift+Left/Right to move position COLUMN?
                                // Let's check `app.move_position_col()`. It toggles Call/Put.
                                // The User Request says: 
                                // "switching to future expiries... shift + →"
                                // "switching to past expiries... shift + ←"
                                // This CONFLICTS with existing "Move Position" shift+arrow logic if mapped to Left/Right.
                                // Existing `move_position_col`: toggles between Call/Put column on the SAME row.
                                // The user explicitly asked for Shift+Arrows for expiry.
                                // I should probably prioritize the User's NEW request for Shift+Left/Right.
                                // But wait, how do I move position column then?
                                // Maybe Shift+Up/Down is enough for row?
                                // Let's check `move_position_col`. It swaps Call/Put for the *selected position*. 
                                // Maybe I should remap `move_position_col` or just let Expiry take precedence?
                                // OR:
                                // Maybe the user wants Shift+Left/Right for Expiry, and existing Move Position usage needs to change?
                                // The user said: "switching to future expiries ...: shift + →".
                                // I will implement Shift+Left/Right for Expiry switching.
                                // I will REMOVE the `move_position_col` binding from Shift+Left/Right.
                                // To Keep `move_position_col` accessible, maybe I just map it to something else? 
                                // Actually `move_position_col` toggles `selected_column`.
                                // Let's see... `app.move_position_col()` is used to *move the selection* or *move the position*?
                                // It seems `move_position_col` does `toggle_column` AND updates position kind.
                                // If I assign Shift+Left/Right to Expiry, I lose the ability to "Flip" a position from Call to Put using keyboard.
                                // That seems acceptable if not mentioned, or I can map it to something else.
                                // But wait, standard navigation (no shift) uses Left/Right for `toggle_column`.
                                // Shift+Left/Right was `move_position_col`.
                                // I will overwrite it.
                                
                                if app.previous_expiry() {
                                    // Send update
                                    if let Some(exp) = app.available_expiries.get(app.current_expiry_index) {
                                        let _ = expiry_tx.send(exp.clone());
                                        // Clear data potentially to avoid showing stale data for wrong expiry?
                                        // Better to let next fetch handle it.
                                        app.data.clear(); 
                                    }
                                }
                            },
                            KeyCode::Right => { 
                                if app.next_expiry() {
                                    if let Some(exp) = app.available_expiries.get(app.current_expiry_index) {
                                        let _ = expiry_tx.send(exp.clone());
                                        app.data.clear();
                                    }
                                }
                            },
                            KeyCode::Char('S') | KeyCode::Char('s') => app.toggle_help(),
                            _ => {}
                        }
                    } else {
                        match key.code {
                            KeyCode::Tab | KeyCode::BackTab => {
                                app.active_focus = match app.active_focus {
                                    app::Focus::OptionChain => app::Focus::Strategies,
                                    app::Focus::Strategies => app::Focus::OptionChain,
                                };
                            },
                            _ => {
                                match app.active_focus {
                                    app::Focus::OptionChain => {
                                        match key.code {
                                            KeyCode::Char('q') => app.should_quit = true,
                                            KeyCode::Char('b') | KeyCode::Char('B') => app.handle_trade_action(true),
                                            KeyCode::Char('s') | KeyCode::Char('S') => app.handle_trade_action(false),
                                            KeyCode::Char(' ') => app.toggle_selection(),
                                            KeyCode::Delete | KeyCode::Backspace => app.delete_position(),
                                            KeyCode::Down => app.next_row(),
                                            KeyCode::Up => app.previous_row(),
                                            KeyCode::Left | KeyCode::Right => app.toggle_column(),
                                            KeyCode::Char('?') => app.toggle_help(),
                                            _ => {}
                                        }
                                    },
                                    app::Focus::Strategies => {
                                        match key.code {
                                            KeyCode::Char('q') => app.should_quit = true,
                                            KeyCode::Down => app.next_strategy(),
                                            KeyCode::Up => app.previous_strategy(),
                                            KeyCode::Char('?') => app.toggle_help(),
                                            _ => {}
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            app.on_tick();
            last_tick = std::time::Instant::now();
        }

        // Check for new data
        if let Ok(new_data) = rx.try_recv() {
            app.data = new_data;
            app.update_live_prices();
                        
            // Auto-center on ATM if first load
            if !app.initial_centering_done && !app.data.is_empty() {
                let spot_price = app.data.first().map(|d| d.underlying_spot_price).unwrap_or(0.0);
                let closest = app.data.iter().enumerate().min_by(|(_, a), (_, b)| {
                    let diff_a = (a.strike_price - spot_price).abs();
                    let diff_b = (b.strike_price - spot_price).abs();
                    diff_a.partial_cmp(&diff_b).unwrap_or(std::cmp::Ordering::Equal)
                }).map(|(i, _)| i);

                if let Some(idx) = closest {
                    app.selected_row = idx;
                    app.initial_centering_done = true;
                }
            }

             // Ensure selection stays within bounds if data shrinks
            if app.selected_row >= app.data.len() {
                app.selected_row = app.data.len().saturating_sub(1);
            }
        }

        if app.should_quit {
            break;
        }
    }

    // 6. Restore Terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}



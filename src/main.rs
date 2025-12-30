use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io::{self, Write}, time::Duration};
use tokio::sync::mpsc;

mod app;
mod model;
mod ui;
mod strategy;

use app::App;
use model::ApiResponse;

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Get Access Token
    print!("Enter Upstox Access Token: ");
    io::stdout().flush()?;
    let mut token = String::new();
    io::stdin().read_line(&mut token)?;
    let token = token.trim().to_string();

    // 2. Setup Terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // 3. Setup App and Data Channel
    let mut app = App::new();
    let (tx, mut rx) = mpsc::channel(10);

    // 4. Background Data Fetcher
    let token_clone = token.clone();
    tokio::spawn(async move {
        let client = reqwest::Client::new();
        let url = "https://api.upstox.com/v2/option/chain?instrument_key=NSE_INDEX%7CNifty%2050&expiry_date=2026-01-06";
        
        loop {
            let res = client
                .get(url)
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
                    // Send empty or log error? For now just ignore valid fetch errors to retry
                     eprintln!("Error fetching data: {}", e);
                }
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    });

    // 5. Main Event Loop
    let tick_rate = Duration::from_millis(250);
    let mut last_tick = std::time::Instant::now();

    loop {
        terminal.draw(|f| ui::draw(f, &app))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == event::KeyEventKind::Press {
                    if key.modifiers.contains(event::KeyModifiers::SHIFT) {
                        match key.code {
                            KeyCode::Down => app.move_position_row(1),
                            KeyCode::Up => app.move_position_row(-1),
                            KeyCode::Left | KeyCode::Right => app.move_position_col(),
                            _ => {}
                        }
                    } else {
                        match key.code {
                            KeyCode::Char('q') => app.should_quit = true,
                            KeyCode::Char('b') | KeyCode::Char('B') => app.handle_trade_action(true),
                            KeyCode::Char('s') | KeyCode::Char('S') => app.handle_trade_action(false),
                            KeyCode::Delete | KeyCode::Backspace => app.delete_position(),
                            KeyCode::Down => app.next_row(),
                            KeyCode::Up => app.previous_row(),
                            KeyCode::Left | KeyCode::Right => app.toggle_column(),
                            _ => {}
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
                    diff_a.partial_cmp(&diff_b).unwrap()
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

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
    // 1. Setup Terminal Early
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // 2. Get Access Token (TUI Mode)
    let mut token = String::new();
    let mut error_msg = String::new();

    loop {
        terminal.draw(|f| {
            let size = f.size();
            let block = ratatui::widgets::Block::default()
                .title(" Setup Trakbit ")
                .borders(ratatui::widgets::Borders::ALL)
                .border_style(ratatui::style::Style::default().fg(ratatui::style::Color::Cyan));
            
            // Center the box
            let area = centered_rect(60, 40, size);
            
            let chunks = ratatui::layout::Layout::default()
                .direction(ratatui::layout::Direction::Vertical)
                .constraints([
                    ratatui::layout::Constraint::Length(4), // Welcome
                    ratatui::layout::Constraint::Length(4), // Link
                    ratatui::layout::Constraint::Length(4), // Steps
                    ratatui::layout::Constraint::Length(3), // Input
                    ratatui::layout::Constraint::Min(2),    // Error/Status
                ])
                .margin(2)
                .split(area);

            f.render_widget(ratatui::widgets::Clear, area); // Clear background
            f.render_widget(block, area);

            let welcome = ratatui::widgets::Paragraph::new(vec![
                ratatui::text::Line::from(ratatui::text::Span::styled("Welcome to Trakbit!", ratatui::style::Style::default().add_modifier(ratatui::style::Modifier::BOLD).fg(ratatui::style::Color::Magenta))),
                ratatui::text::Line::from("To get started, you need an Upstox Access Token."),
            ]).alignment(ratatui::layout::Alignment::Center);

            let link_text = vec![
                ratatui::text::Line::from("1. Go to:"),
                ratatui::text::Line::from(ratatui::text::Span::styled("https://account.upstox.com/developer/apps", ratatui::style::Style::default().fg(ratatui::style::Color::Blue).add_modifier(ratatui::style::Modifier::UNDERLINED))),
            ];
            let link = ratatui::widgets::Paragraph::new(link_text).alignment(ratatui::layout::Alignment::Center);

            let steps_text = vec![
                ratatui::text::Line::from("2. Create a new app (or use existing)"),
                ratatui::text::Line::from("3. Copy 'Access Token' and paste below"),
            ];
            let steps = ratatui::widgets::Paragraph::new(steps_text).alignment(ratatui::layout::Alignment::Center);
            
            let input_block = ratatui::widgets::Block::default()
                .borders(ratatui::widgets::Borders::ALL)
                .title(" Access Token ");
            
            // Mask the token
            let masked_token: String = "*".repeat(token.len());
            // Show only the tail if it doesn't fit
            let inner_width = chunks[3].width.saturating_sub(2) as usize;
            let display_token = if masked_token.len() > inner_width {
                // Show the last N chars
                masked_token.chars().rev().take(inner_width).collect::<String>().chars().rev().collect()
            } else {
                masked_token
            };

            let input = ratatui::widgets::Paragraph::new(display_token)
                .style(ratatui::style::Style::default().fg(ratatui::style::Color::Yellow))
                .block(input_block);

            let status = ratatui::widgets::Paragraph::new(if error_msg.is_empty() { 
                    ratatui::text::Line::from(vec![ratatui::text::Span::raw("Press "), ratatui::text::Span::styled("Enter", ratatui::style::Style::default().add_modifier(ratatui::style::Modifier::BOLD)), ratatui::text::Span::raw(" to continue")])
                } else {
                    ratatui::text::Line::from(ratatui::text::Span::styled(format!("Error: {}", error_msg), ratatui::style::Style::default().fg(ratatui::style::Color::Red)))
                })
                .alignment(ratatui::layout::Alignment::Center);

            f.render_widget(welcome, chunks[0]);
            f.render_widget(link, chunks[1]);
            f.render_widget(steps, chunks[2]);
            f.render_widget(input, chunks[3]);
            f.render_widget(status, chunks[4]);

        })?;

        let mut should_break = false;
        if event::poll(Duration::from_millis(100))? {
            // Consume all available events to handle paste
            loop {
                // Check if there is an event to read
                if !event::poll(Duration::from_millis(0))? {
                    break;
                }
                
                if let Event::Key(key) = event::read()? {
                     if key.kind == event::KeyEventKind::Press {
                        match key.code {
                            KeyCode::Char(c) => {
                                token.push(c);
                                error_msg.clear();
                            },
                            KeyCode::Backspace => {
                                token.pop();
                                error_msg.clear();
                            },
                            KeyCode::Enter => {
                                if token.trim().is_empty() {
                                    error_msg = "Token cannot be empty".to_string();
                                } else {
                                    should_break = true;
                                }
                            },
                            KeyCode::Esc => {
                                 // Exit gracefully
                                disable_raw_mode()?;
                                execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
                                terminal.show_cursor()?;
                                return Ok(());
                            }
                            _ => {}
                        }
                     }
                }
                
                if should_break {
                    break;
                }
            }
        }
        
        if should_break {
            break;
        }
    }


    let token = token.trim().to_string();

    // 3. Setup App and Data Channel
    let mut app = App::new();
    let (tx, mut rx) = mpsc::channel(10);

    // 4. Background Data Fetcher
    let token_clone = token.clone();
    tokio::spawn(async move {
        let client = reqwest::Client::new();
        // Assuming user wants NIFTY 50 for now, could also be configurable later
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
        terminal.draw(|f| ui::draw(f, &mut app))?;

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

fn centered_rect(percent_x: u16, percent_y: u16, r: ratatui::layout::Rect) -> ratatui::layout::Rect {
    let popup_layout = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            ratatui::layout::Constraint::Percentage((100 - percent_y) / 2),
            ratatui::layout::Constraint::Percentage(percent_y),
            ratatui::layout::Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Horizontal)
        .constraints([
            ratatui::layout::Constraint::Percentage((100 - percent_x) / 2),
            ratatui::layout::Constraint::Percentage(percent_x),
            ratatui::layout::Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

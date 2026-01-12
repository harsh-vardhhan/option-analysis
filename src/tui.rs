use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io::{self, Stdout}, time::{Duration, Instant}};
use tokio::sync::{mpsc, watch};

use crate::app::{self, App};
use crate::model::OptionData;
use crate::ui;

// [NEW] Enum to handle distinct data update types
pub enum TuiMessage {
    OptionChain(Vec<OptionData>),
    Quote(crate::model::QuoteData),
}

pub struct Tui {
    pub terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl Tui {
    pub fn new() -> Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        Ok(Self { terminal })
    }

    pub async fn run(
        &mut self,
        app: &mut App,
        mut rx: mpsc::Receiver<TuiMessage>,      // Changed from Vec<OptionData>
        expiry_tx: watch::Sender<String>,
        quote_tx: mpsc::Sender<String>           // [NEW] Channel to request quotes
    ) -> Result<()> {
        let tick_rate = Duration::from_millis(250);
        let mut last_tick = Instant::now();

        loop {
            self.terminal.draw(|f| ui::draw(f, app))?;

            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

            if crossterm::event::poll(timeout)? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == event::KeyEventKind::Press {
                        // Capture navigation that changes selection to trigger quote update
                        let mut selection_changed = false;

                        if app.show_help {
                             match key.code {
                                KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') => app.toggle_help(),
                                KeyCode::Char('s') | KeyCode::Char('S') if key.modifiers.contains(KeyModifiers::SHIFT) => app.toggle_help(),
                                _ => {}
                             }
                        } else if key.modifiers.contains(KeyModifiers::SHIFT) {
                            match key.code {
                                KeyCode::Down => app.move_position_row(1),
                                KeyCode::Up => app.move_position_row(-1),
                                KeyCode::Left => {
                                    // Switch to previous expiry
                                    if app.previous_expiry() {
                                        if let Some(exp) = app.available_expiries.get(app.current_expiry_index) {
                                            let _ = expiry_tx.send(exp.clone());
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
                                                KeyCode::Down => {
                                                    app.next_row();
                                                    selection_changed = true;
                                                },
                                                KeyCode::Up => {
                                                    app.previous_row();
                                                    selection_changed = true;
                                                },
                                                KeyCode::Left | KeyCode::Right => {
                                                    app.toggle_column();
                                                    selection_changed = true;
                                                },
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
                        
                        // [NEW] Trigger Quote Fetch if selection changed
                        if selection_changed {
                            if let Some(instr_key) = app.get_selected_instrument_key() {
                                let _ = quote_tx.send(instr_key).await;
                            }
                        }
                    }
                }
            }

            if last_tick.elapsed() >= tick_rate {
                app.on_tick();
                last_tick = Instant::now();
            }

            // Check for new data
            if let Ok(msg) = rx.try_recv() {
                match msg {
                    TuiMessage::OptionChain(new_data) => {
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

                        if let Some(instr_key) = app.get_selected_instrument_key() {
                            let _ = quote_tx.send(instr_key).await;
                        }
                    },
                    TuiMessage::Quote(quote_data) => {
                        app.market_depth = Some(quote_data);
                    }
                }
            }

            if app.should_quit {
                break;
            }
        }
        Ok(())
    }
}

impl Drop for Tui {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        );
        let _ = self.terminal.show_cursor();
    }
}

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io, time::Duration};

pub enum SetupResult {
    Token(String),
    Demo,
}

pub async fn run_setup_tui(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    validation_url: &str,
) -> Result<SetupResult> {
    let mut token = String::new();
    let mut error_msg = String::new();
    let mut is_validating = false;

    loop {
        terminal.draw(|f| {
            let size = f.size();
            let block = ratatui::widgets::Block::default()
                .title(" Setup Trakbit ")
                .borders(ratatui::widgets::Borders::ALL)
                .border_style(ratatui::style::Style::default().fg(ratatui::style::Color::Cyan));
            
            // Center the box - increased to 80/80 to prevent squashing on small terminals
            let area = crate::ui::centered_rect(80, 80, size);
            
            let chunks = ratatui::layout::Layout::default()
                .direction(ratatui::layout::Direction::Vertical)
                .constraints([
                    ratatui::layout::Constraint::Length(4), // Welcome
                    ratatui::layout::Constraint::Length(4), // Link
                    ratatui::layout::Constraint::Length(4), // Steps
                    ratatui::layout::Constraint::Length(3), // Input
                    ratatui::layout::Constraint::Length(2), // Demo info
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
            // Safe width calc
            let inner_width = chunks[3].width.saturating_sub(2) as usize;
            let display_token = if inner_width == 0 {
                String::new()
            } else if masked_token.len() > inner_width {
                // Show the last N chars
                masked_token.chars().rev().take(inner_width).collect::<String>().chars().rev().collect()
            } else {
                masked_token
            };

            let input = ratatui::widgets::Paragraph::new(display_token)
                .style(ratatui::style::Style::default().fg(ratatui::style::Color::Yellow))
                .block(input_block);

            let demo_info = ratatui::widgets::Paragraph::new(
                ratatui::text::Line::from(vec![ratatui::text::Span::raw("Or press "), ratatui::text::Span::styled("D", ratatui::style::Style::default().add_modifier(ratatui::style::Modifier::BOLD)), ratatui::text::Span::raw(" for Demo Mode")])
            ).alignment(ratatui::layout::Alignment::Center);

            let status_text = if is_validating {
                ratatui::text::Line::from(ratatui::text::Span::styled("Validating token...", ratatui::style::Style::default().fg(ratatui::style::Color::Yellow)))
            } else if error_msg.is_empty() {
                ratatui::text::Line::from(vec![ratatui::text::Span::raw("Press "), ratatui::text::Span::styled("Enter", ratatui::style::Style::default().add_modifier(ratatui::style::Modifier::BOLD)), ratatui::text::Span::raw(" to continue")])
            } else {
                ratatui::text::Line::from(ratatui::text::Span::styled(format!("Error: {}", error_msg), ratatui::style::Style::default().fg(ratatui::style::Color::Red)))
            };

            let status = ratatui::widgets::Paragraph::new(status_text)
                .alignment(ratatui::layout::Alignment::Center);

            f.render_widget(welcome, chunks[0]);
            f.render_widget(link, chunks[1]);
            f.render_widget(steps, chunks[2]);
            f.render_widget(input, chunks[3]);
            f.render_widget(demo_info, chunks[4]);
            f.render_widget(status, chunks[5]);

        })?;

        // If validating, logic is handled below after drawing "Validating..."
        // But we need to actually do the validation now if we set the flag in the previous loop iteration?
        // No, let's do it inline to keep it simple, but we need to re-draw for "Validating" to show up.
        // So we can set is_validating = true, re-draw, then validate.
        
        if is_validating {
            // Perform validation
             let client = reqwest::Client::new();
             let res = client.get(validation_url)
                .header("Content-Type", "application/json")
                .header("Accept", "application/json")
                .header("Authorization", format!("Bearer {}", token.trim()))
                .send()
                .await;

            is_validating = false; // Reset flag

            match res {
                Ok(response) => {
                    if response.status().is_success() {
                        // Check if data is not empty / valid API response structure if needed
                        // But status 200 is usually enough for auth check.
                        // Ideally we check if it returns proper JSON, but let's assume 200 means auth worked.
                        break; 
                    } else if response.status() == reqwest::StatusCode::UNAUTHORIZED {
                         error_msg = "Invalid Access Token".to_string();
                    } else {
                         error_msg = format!("API Error: {}", response.status());
                    }
                },
                Err(e) => {
                    error_msg = format!("Network/Request Error: {}", e);
                }
            }
            continue; // Re-loop to show result
        }


        let mut should_validate = false;

        if event::poll(Duration::from_millis(100))? {
            // Read first event strictly
            let first_event = event::read()?;
            let mut events = vec![first_event];
            
            // Collect any other immediately available events (paste)
            while event::poll(Duration::from_millis(0))? {
                events.push(event::read()?);
            }

            for e in events {
                if let Event::Key(key) = e {
                    if key.kind == event::KeyEventKind::Press {
                        match key.code {
                            KeyCode::Char('d') | KeyCode::Char('D') if token.trim().is_empty() => {
                                return Ok(SetupResult::Demo);
                            },
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
                                    should_validate = true;
                                }
                            },
                            KeyCode::Esc => {
                                disable_raw_mode()?;
                                execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
                                terminal.show_cursor()?;
                                return Err(anyhow::anyhow!("User cancelled setup"));
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        
        if should_validate {
            is_validating = true;
            // Next loop iteration will draw "Validating..." and then perform the check
        }
    }

    Ok(SetupResult::Token(token.trim().to_string()))
}

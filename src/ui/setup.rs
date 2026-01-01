use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io, time::Duration};

pub fn run_setup_tui(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<String> {
    let mut token = String::new();
    let mut error_msg = String::new();

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
                                disable_raw_mode()?;
                                execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
                                terminal.show_cursor()?;
                                // We return an error or handle exit. 
                                // Since we are in run_setup_tui, maybe we should return Result<Option<String>>?
                                // Or we can just panic/exit. The original code just returned Ok(()).
                                // But here main expects a token.
                                // Let's exit the process? Or return Error.
                                return Err(anyhow::anyhow!("User cancelled setup"));
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        
        if should_break {
            break;
        }
    }

    Ok(token.trim().to_string())
}

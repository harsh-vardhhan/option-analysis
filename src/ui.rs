use ratatui::{
    layout::{Constraint, Layout, Alignment},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState},
    Frame,
};

use crate::app::{App, ColumnSelection};

pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .constraints([Constraint::Length(4), Constraint::Min(0)])
        .split(f.size());

    // --- DASHBOARD ---
    let spot_price = app.data.first().map(|d| d.underlying_spot_price).unwrap_or(0.0);
    // Assuming underlying key has the name, e.g., "NSE_INDEX|Nifty 50"
    let underlying = app.data.first()
        .map(|d| d.underlying_key.split('|').last().unwrap_or(&d.underlying_key))
        .unwrap_or("NIFTY");
    let expiry = app.data.first().map(|d| d.expiry.as_str()).unwrap_or("-");

    let dashboard_text = vec![
        Line::from(vec![
            Span::raw("Symbol: "),
            Span::styled(underlying, Style::default().add_modifier(Modifier::BOLD).fg(Color::Yellow)),
            Span::raw("  |  Spot Price: "),
            Span::styled(format!("{:.2}", spot_price), Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan)),
            Span::raw("  |  Expiry: "),
            Span::styled(expiry, Style::default().fg(Color::Magenta)),
        ]),
        Line::from(vec![
             Span::styled("LIVE MARKET DATA", Style::default().fg(Color::Green).add_modifier(Modifier::RAPID_BLINK)), // subtle blinking
             Span::raw(" • Press 'q' to quit • Arrow keys to navigate"),
        ])
    ];

    let dashboard = Paragraph::new(dashboard_text)
        .block(Block::default().borders(Borders::ALL).title(" Dashboard "))
        .alignment(Alignment::Center);
    f.render_widget(dashboard, chunks[0]);

    if app.data.is_empty() {
        return;
    }

    // --- TABLE ---

    // Find ATM strike (closest to spot)
    let closest_strike_index = app.data
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            let diff_a = (a.strike_price - spot_price).abs();
            let diff_b = (b.strike_price - spot_price).abs();
            diff_a.partial_cmp(&diff_b).unwrap()
        })
        .map(|(i, _)| i);

    let rows = app.data.iter().enumerate().map(|(i, item)| {
        let call_ltp = item.call_options.as_ref().map(|o| o.market_data.ltp).unwrap_or(0.0);
        let put_ltp = item.put_options.as_ref().map(|o| o.market_data.ltp).unwrap_or(0.0);

        // Base styles for ITM/OTM
        // Professional Theme:
        // OTM: Default/Grayish
        // ITM Call: Subtle Green Background
        // ITM Put: Subtle Red Background
        
        // Define colors
        let itm_call_bg = Color::Rgb(15, 40, 15);
        let itm_put_bg = Color::Rgb(40, 15, 15);
        let text_color = Color::White;
        let dim_text_color = Color::Rgb(150, 150, 150);

        let mut call_style = Style::default().fg(text_color);
        let mut put_style = Style::default().fg(text_color);
        let mut strike_style = Style::default().fg(Color::Yellow); // Strikes usually distinct

        // ITM Logic
        if item.strike_price < spot_price {
            // Call ITM
            call_style = call_style.bg(itm_call_bg);
        } else {
            // Call OTM
            call_style = call_style.fg(dim_text_color);
        }

        if item.strike_price > spot_price {
            // Put ITM
            put_style = put_style.bg(itm_put_bg);
        } else {
            // Put OTM
            put_style = put_style.fg(dim_text_color);
        }

        // ATM Logic
        if Some(i) == closest_strike_index {
             strike_style = strike_style.bg(Color::Blue).fg(Color::White).add_modifier(Modifier::BOLD);
        }

        // Selection Highlight
        // High Contrast for selected row
        if i == app.selected_row {
            let sel_bg = Color::White;
            let sel_fg = Color::Black;

            match app.selected_column {
                ColumnSelection::Call => {
                    call_style = call_style.bg(sel_bg).fg(sel_fg).add_modifier(Modifier::BOLD);
                }
                ColumnSelection::Put => {
                    put_style = put_style.bg(sel_bg).fg(sel_fg).add_modifier(Modifier::BOLD);
                }
            }
            // Add a subtle highlight to the non-selected column in the same row to track the eye
            if strike_style.bg != Some(Color::Blue) {
               strike_style = strike_style.bg(Color::DarkGray);
            }
        }

        Row::new(vec![
            Cell::from(Line::from(format!("{:.2}", call_ltp)).alignment(Alignment::Right)).style(call_style),
            Cell::from(Line::from(format!("{:.0}", item.strike_price)).alignment(Alignment::Center)).style(strike_style),
            Cell::from(Line::from(format!("{:.2}", put_ltp)).alignment(Alignment::Left)).style(put_style),
        ])
    });

    let table = Table::new(rows, [
        Constraint::Percentage(40),
        Constraint::Length(15), // Center column fixed width
        Constraint::Percentage(40),
    ])
    .header(
        Row::new(vec![
            Cell::from(Line::from("CALLS").alignment(Alignment::Right)).style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Cell::from(Line::from("STRIKE").alignment(Alignment::Center)).style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Cell::from(Line::from("PUTS").alignment(Alignment::Left)).style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
        ])
        .bottom_margin(1)
        .height(1)
    )
    .column_spacing(2);

    let mut table_state = TableState::default();
    table_state.select(Some(app.selected_row));

    f.render_stateful_widget(table, chunks[1], &mut table_state);
}

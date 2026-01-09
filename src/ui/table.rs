use ratatui::{
    layout::{Constraint, Alignment},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Row, Table},
    Frame,
    layout::Rect,
};
use crate::app::{App, ColumnSelection};

pub fn draw(f: &mut Frame, app: &mut App, area: Rect) {


    let spot_price = app.data.first().map(|d| d.underlying_spot_price).unwrap_or(0.0);

    // Find ATM strike (closest to spot)
    let closest_strike_index = app.data
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            let diff_a = (a.strike_price - spot_price).abs();
            let diff_b = (b.strike_price - spot_price).abs();
            diff_a.partial_cmp(&diff_b).unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(i, _)| i);

    // Calculate Max OI for scaling
    let max_oi = app.data.iter().fold(0.0f64, |acc, item| {
        let call_oi = item.call_options.as_ref().map(|o| o.market_data.oi).unwrap_or(0.0);
        let put_oi = item.put_options.as_ref().map(|o| o.market_data.oi).unwrap_or(0.0);
        acc.max(call_oi).max(put_oi)
    });
    
    // Calculate available width for OI columns
    // Total width - Fixed columns (LTP 22*2 + Strike 10 + Delta 8*2 = 70) - Borders (2) - Spacing (6) = 78 overhead
    let overhead = 78;
    let available_width = area.width.saturating_sub(overhead) as usize;
    let max_bar_width = (available_width / 2).max(10); // Ensure at least 10 chars

    // Bar drawing helper
    let draw_bar = |val: f64, max: f64, color: Color, grow_left: bool| -> Line {
        if max == 0.0 { return Line::from(" ".repeat(max_bar_width)); }
        let width = max_bar_width; 
        // User request: Highest OI should be 90% of the width
        let ratio = (val / max).min(1.0);
        let filled = (ratio * width as f64 * 0.9).round() as usize; 
        let empty = width - filled;
        
        let bar_char = "▆"; 
        let bar_str = bar_char.repeat(filled);
        let empty_str = " ".repeat(empty);
        
        let spans = if grow_left {
             // Grow Left <- (e.g. "   |||")
             vec![
                 Span::raw(empty_str),
                 Span::styled(bar_str, Style::default().fg(color).add_modifier(Modifier::DIM)),
             ]
        } else {
             // Grow Right -> (e.g. "|||   ")
             vec![
                 Span::styled(bar_str, Style::default().fg(color).add_modifier(Modifier::DIM)),
                 Span::raw(empty_str),
             ]
        };
        
        Line::from(spans)
    };

    let rows = app.data.iter().enumerate().map(|(i, item)| {
        let call_ltp = item.call_options.as_ref().map(|o| o.market_data.ltp).unwrap_or(0.0);
        let put_ltp = item.put_options.as_ref().map(|o| o.market_data.ltp).unwrap_or(0.0);
        let call_oi = item.call_options.as_ref().map(|o| o.market_data.oi).unwrap_or(0.0);
        let put_oi = item.put_options.as_ref().map(|o| o.market_data.oi).unwrap_or(0.0);

        let itm_call_bg = Color::Rgb(15, 40, 15);
        let itm_put_bg = Color::Rgb(40, 15, 15);
        let text_color = Color::White;
        let dim_text_color = Color::Rgb(150, 150, 150);

        let mut call_style = Style::default().fg(text_color);
        let mut put_style = Style::default().fg(text_color);
        let mut strike_style = Style::default().fg(Color::Yellow);

        let is_call_itm = item.strike_price < spot_price;
        let is_put_itm = item.strike_price > spot_price;

        if is_call_itm { call_style = call_style.bg(itm_call_bg); } else { call_style = call_style.fg(dim_text_color); }
        if is_put_itm { put_style = put_style.bg(itm_put_bg); } else { put_style = put_style.fg(dim_text_color); }

        if Some(i) == closest_strike_index {
             strike_style = strike_style.bg(Color::Blue).fg(Color::White).add_modifier(Modifier::BOLD);
        }

        if i == app.selected_row {
            let sel_bg = Color::White;
            let sel_fg = Color::Black;
            match app.selected_column {
                ColumnSelection::Call => { call_style = call_style.bg(sel_bg).fg(sel_fg).add_modifier(Modifier::BOLD); }
                ColumnSelection::Put => { put_style = put_style.bg(sel_bg).fg(sel_fg).add_modifier(Modifier::BOLD); }
            }
            if strike_style.bg != Some(Color::Blue) {
               strike_style = strike_style.bg(Color::DarkGray);
            }
        }

        // Check selection for Call
        // Note: Using format!("{:.0}") might be risky if we stored {:.2} in App. 
        // Let's verify App usage. App uses {:.2}. 
        // Table display uses {:.0} for strike text, but the data is f64.
        // Let's match the key format used in App: format!("{:.2}", item.strike_price)
        let call_key_exact = (format!("{:.2}", item.strike_price), crate::strategy::OptionType::Call);
        if app.selected_positions.contains(&call_key_exact) {
            call_style = call_style.bg(Color::DarkGray).add_modifier(Modifier::UNDERLINED);
        }

        let put_key_exact = (format!("{:.2}", item.strike_price), crate::strategy::OptionType::Put);
        if app.selected_positions.contains(&put_key_exact) {
             put_style = put_style.bg(Color::DarkGray).add_modifier(Modifier::UNDERLINED);
        }

        // Apply Selection Style Override if this row is selected (re-apply to ensure it's on top of ITM colors)
        // Actually, we already did this above. The issue is `if i == app.selected_row` block is duplicated in original code.
        // I will just rely on the first block.

        let call_qty = app.portfolio.positions.iter().find(|p| p.strike == item.strike_price && p.kind == crate::strategy::OptionType::Call).map(|p| p.qty).unwrap_or(0);
        let put_qty = app.portfolio.positions.iter().find(|p| p.strike == item.strike_price && p.kind == crate::strategy::OptionType::Put).map(|p| p.qty).unwrap_or(0);

        // Helper to create badge spans
        let create_badge = |qty: i32| -> Span {
            let is_buy = qty > 0;
            let label = if is_buy { "BUY" } else { "SELL" };
            let sign = if is_buy { "+" } else { "-" };
            let color = if is_buy { Color::Green } else { Color::Red };
            
            Span::styled(
                format!(" {}{} {} ", sign, qty.abs(), label), 
                Style::default().bg(color).fg(Color::Black).add_modifier(Modifier::BOLD)
            )
        };

        let call_content = if call_qty != 0 {
            vec![
                create_badge(call_qty),
                Span::raw(" "),
                Span::raw(format!("{:.2}", call_ltp))
            ]
        } else {
            vec![Span::raw(format!("{:.2}", call_ltp))]
        };

        let put_content = if put_qty != 0 {
            vec![
                Span::raw(format!("{:.2}", put_ltp)),
                Span::raw(" "),
                create_badge(put_qty)
            ]
        } else {
            vec![Span::raw(format!("{:.2}", put_ltp))]
        };
        
        // Get Deltas (Prefer API value)
        let call_delta = item.call_options.as_ref().and_then(|o| o.option_greeks.as_ref()).map(|g| g.delta).unwrap_or(0.0);
        let put_delta = item.put_options.as_ref().and_then(|o| o.option_greeks.as_ref()).map(|g| g.delta).unwrap_or(0.0);

        // Point Inwards:
        // Left Column (Call OI): Grow Right -> (grow_left = false)
        // Right Column (Put OI): Grow Left <- (grow_left = true)
        Row::new(vec![
            Cell::from(draw_bar(call_oi, max_oi, Color::Green, false).alignment(Alignment::Left)).style(call_style),
            Cell::from(Line::from(format!("{:.2}", call_delta)).alignment(Alignment::Center)).style(call_style),
            Cell::from(Line::from(call_content).alignment(Alignment::Right)).style(call_style),
            Cell::from(Line::from(format!("{:.0}", item.strike_price)).alignment(Alignment::Center)).style(strike_style),
            Cell::from(Line::from(put_content).alignment(Alignment::Left)).style(put_style),
            Cell::from(Line::from(format!("{:.2}", put_delta)).alignment(Alignment::Center)).style(put_style),
            Cell::from(draw_bar(put_oi, max_oi, Color::Red, true).alignment(Alignment::Right)).style(put_style),
        ])
    });

    // Determine Border Color based on Focus
    let border_color = if app.active_focus == crate::app::Focus::OptionChain {
        Color::Cyan
    } else {
        Color::DarkGray
    };

    let table = Table::new(rows, [
        Constraint::Min(8),         // Call OI
        Constraint::Length(8),      // Call Delta
        Constraint::Length(22),     // Call LTP: Increased to fit badges
        Constraint::Length(10),     // Strike
        Constraint::Length(22),     // Put LTP: Increased to fit badges
        Constraint::Length(8),      // Put Delta
        Constraint::Min(8),         // Put OI
    ])
    .header(
        Row::new(vec![
            Cell::from(Line::from("OI").alignment(Alignment::Left)).style(Style::default().fg(Color::Green)),
            Cell::from(Line::from("Delta").alignment(Alignment::Center)).style(Style::default().fg(Color::Green)), // slightly simpler color
            Cell::from(Line::from("CALLS").alignment(Alignment::Right)).style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Cell::from(Line::from("STRIKE").alignment(Alignment::Center)).style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Cell::from(Line::from("PUTS").alignment(Alignment::Left)).style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Cell::from(Line::from("Delta").alignment(Alignment::Center)).style(Style::default().fg(Color::Red)),
            Cell::from(Line::from("OI").alignment(Alignment::Right)).style(Style::default().fg(Color::Red)),
        ])
        .bottom_margin(1)
        .height(1)
    )
    .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(border_color)).title(" Option Chain "))
    .column_spacing(1);

    // Sync selection state with persisted TableState
    app.table_state.select(Some(app.selected_row));

    f.render_stateful_widget(table, area, &mut app.table_state);
}

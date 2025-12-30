use ratatui::{
    layout::{Constraint, Layout, Alignment, Direction},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState, Chart, Axis, Dataset, GraphType},
    symbols,
    Frame,
};

use crate::app::{App, ColumnSelection};

pub fn draw(f: &mut Frame, app: &App) {
    let constraints = vec![
        Constraint::Length(4), 
        Constraint::Min(0), 
        Constraint::Length(15)
    ];

    let chunks = Layout::default()
        .constraints(constraints)
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
            Span::styled(format!(" {} ", underlying), Style::default().bg(Color::Blue).fg(Color::White).add_modifier(Modifier::BOLD)),
            Span::raw(" "),
            Span::styled(format!(" {:.2} ", spot_price), Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw("  Expiry: "),
            Span::styled(expiry, Style::default().fg(Color::Yellow)),
            Span::raw("  |  Last Action: "),
            Span::styled(format!(" {} ", app.last_message), Style::default().bg(Color::DarkGray).fg(Color::White)),
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

    // Calculate Max OI for scaling
    let max_oi = app.data.iter().fold(0.0f64, |acc, item| {
        let call_oi = item.call_options.as_ref().map(|o| o.market_data.oi).unwrap_or(0.0);
        let put_oi = item.put_options.as_ref().map(|o| o.market_data.oi).unwrap_or(0.0);
        acc.max(call_oi).max(put_oi)
    });
    
    // Bar drawing helper
    let draw_bar = |val: f64, max: f64, color: Color, grow_left: bool| -> Line {
        if max == 0.0 { return Line::from("        "); }
        let width = 8; // Compact fixed width
        let ratio = (val / max).min(1.0);
        let filled = (ratio * width as f64).round() as usize;
        let empty = width - filled;
        
        let bar_char = "▆"; 
        let bar_str = bar_char.repeat(filled);
        let empty_str = " ".repeat(empty);
        
        let spans = if grow_left {
             // Grow Left <- (e.g. "   |||")
             vec![
                 Span::raw(empty_str),
                 Span::styled(bar_str, Style::default().fg(color)),
             ]
        } else {
             // Grow Right -> (e.g. "|||   ")
             vec![
                 Span::styled(bar_str, Style::default().fg(color)),
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

        // ... (Styles)
        // Re-declaring styles locally since this is a clean replacement block
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

        let call_qty = app.positions.iter().find(|p| p.strike == item.strike_price && p.kind == crate::strategy::OptionType::Call).map(|p| p.qty).unwrap_or(0);
        let put_qty = app.positions.iter().find(|p| p.strike == item.strike_price && p.kind == crate::strategy::OptionType::Put).map(|p| p.qty).unwrap_or(0);

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

        // Point Inwards:
        // Left Column (Call OI): Grow Right -> (grow_left = false)
        // Right Column (Put OI): Grow Left <- (grow_left = true)
        Row::new(vec![
            Cell::from(draw_bar(call_oi, max_oi, Color::Green, false).alignment(Alignment::Left)).style(call_style),
            Cell::from(Line::from(call_content).alignment(Alignment::Right)).style(call_style),
            Cell::from(Line::from(format!("{:.0}", item.strike_price)).alignment(Alignment::Center)).style(strike_style),
            Cell::from(Line::from(put_content).alignment(Alignment::Left)).style(put_style),
            Cell::from(draw_bar(put_oi, max_oi, Color::Red, true).alignment(Alignment::Right)).style(put_style),
        ])
    });

    let table = Table::new(rows, [
        Constraint::Min(10),        // Call OI: Flexible, at least 10
        Constraint::Length(20),     // Call LTP: Fixed reasonable width
        Constraint::Length(12),     // Strike: Fixed
        Constraint::Length(20),     // Put LTP: Fixed
        Constraint::Min(10),        // Put OI: Flexible
    ])
    .header(
        Row::new(vec![
            Cell::from(Line::from("OI").alignment(Alignment::Left)).style(Style::default().fg(Color::Green)),
            Cell::from(Line::from("CALLS").alignment(Alignment::Right)).style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Cell::from(Line::from("STRIKE").alignment(Alignment::Center)).style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Cell::from(Line::from("PUTS").alignment(Alignment::Left)).style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Cell::from(Line::from("OI").alignment(Alignment::Right)).style(Style::default().fg(Color::Red)),
        ])
        .bottom_margin(1)
        .height(1)
    )
    .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)).title(" Option Chain "))
    .column_spacing(1);

    let mut table_state = TableState::default();
    table_state.select(Some(app.selected_row));

    f.render_stateful_widget(table, chunks[1], &mut table_state);

    // --- STRATEGY PANEL ---
    // --- STRATEGY PANEL ---
    if chunks.len() > 2 {
            // Split the bottom chunk into Left (Stats) and Right (Graph)
            let strategy_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(chunks[2]);

        // Calculate Stats
        use crate::strategy::analyze_strategy;
        let stats = analyze_strategy(&app.positions, spot_price);

        let mut text = vec![
            Line::from(Span::styled(" Strategy Builder ", Style::default().add_modifier(Modifier::BOLD).bg(Color::Blue).fg(Color::White))),
            Line::from(""),
        ];

        // List Legs (Compact)
        text.push(Line::from(Span::styled("Active Legs:", Style::default().add_modifier(Modifier::UNDERLINED))));
        if app.positions.is_empty() {
            text.push(Line::from(Span::styled(" No active positions.", Style::default().fg(Color::DarkGray))));
            text.push(Line::from(Span::styled(" Select strikes and press B/S to build.", Style::default().fg(Color::DarkGray))));
        }
        for pos in &app.positions {
            let side = if pos.qty > 0 { "BUY" } else { "SELL" };
            let color = if pos.qty > 0 { Color::Green } else { Color::Red };
            let kind = match pos.kind {
                    crate::strategy::OptionType::Call => "CE",
                    crate::strategy::OptionType::Put => "PE",
            };
            text.push(Line::from(vec![
                Span::styled(format!(" {:<4} ", side), Style::default().bg(color).fg(Color::Black)),
                Span::raw(format!(" {} {} @ {:.1}", pos.qty.abs(), kind, pos.entry_price)),
                Span::styled(format!("  Str: {:.0}", pos.strike), Style::default().fg(Color::Yellow)),
            ]));
        }
        
        let block = Block::default().borders(Borders::ALL).title(" Analysis ");
        if !app.positions.is_empty() {
            // text.push(Line::from(""));
            text.push(Line::from(Span::styled("Analysis:", Style::default().add_modifier(Modifier::UNDERLINED))));
            
            let max_profit_s = if stats.max_profit_unlimited {
                 Span::styled("Unlimited", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
            } else {
                 Span::styled(format!("₹{:.0}", stats.max_profit), Style::default().fg(Color::Green))
            };

            let max_loss_s = if stats.max_loss_unlimited {
                 Span::styled("Unlimited", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
            } else {
                 Span::styled(format!("₹{:.0}", stats.max_loss), Style::default().fg(Color::Red))
            };

            text.push(Line::from(vec![
                 Span::raw("Max Profit: "),
                 max_profit_s,
            ]));
            text.push(Line::from(vec![
                 Span::raw("Max Loss:   "),
                 max_loss_s,
            ]));

            if !stats.breakevens.is_empty() {
                    let be_str: Vec<String> = stats.breakevens.iter().map(|b| format!("{:.0}", b)).collect();
                    text.push(Line::from(vec![
                        Span::raw("Breakeven:  "),
                        Span::styled(be_str.join(", "), Style::default().fg(Color::Cyan)),
                    ]));
            }
        }
        let paragraph = Paragraph::new(text).block(block);
        f.render_widget(paragraph, strategy_chunks[0]);

        // --- GRAPH ---
        // Default bounds if empty: Spot +/- 5%
        let x_min = stats.points.first().map(|p| p.0).unwrap_or(spot_price * 0.95);
        let x_max = stats.points.last().map(|p| p.0).unwrap_or(spot_price * 1.05);

        let x_labels = vec![
            Span::styled(format!("{:.0}", x_min), Style::default().fg(Color::Gray)),
            Span::styled(format!("{:.0}", x_max), Style::default().fg(Color::Gray)),
        ];
        
        let y_min = stats.max_loss.min(0.0) * 1.1; // margin
        let y_max = stats.max_profit.max(0.0) * 1.1;
        
        // If empty, y_min/max are 0.0. Give small range [-1000, 1000] for grid
        let (y_min, y_max) = if app.positions.is_empty() { (-1000.0, 1000.0) } else { (y_min, y_max) };

        let datasets = vec![
            Dataset::default()
                .name("P&L")
                .marker(symbols::Marker::Braille)
                .graph_type(GraphType::Line)
                .style(Style::default().fg(Color::Yellow))
                .data(&stats.points),
        ];
        
        let chart = Chart::new(datasets)
            .block(Block::default().title(" Payoff Graph ").borders(Borders::ALL))
            .x_axis(Axis::default()
                .title("Spot Price")
                .style(Style::default().fg(Color::Gray))
                .bounds([x_min, x_max])
                .labels(x_labels))
            .y_axis(Axis::default()
                .title("P&L")
                .style(Style::default().fg(Color::Gray))
                .bounds([y_min, y_max])
                .labels(vec![
                    Span::styled(format!("{:.0}", y_min), Style::default().fg(Color::Gray)),
                    Span::styled("0", Style::default().fg(Color::Gray)),
                    Span::styled(format!("{:.0}", y_max), Style::default().fg(Color::Gray)),
                ]));
        
        f.render_widget(chart, strategy_chunks[1]);
    }
}

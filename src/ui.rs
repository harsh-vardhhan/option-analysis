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
    let constraints = if app.positions.is_empty() {
        vec![Constraint::Length(4), Constraint::Min(0)]
    } else {
        vec![Constraint::Length(4), Constraint::Min(0), Constraint::Length(15)]
    };

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

        // Check for position on this strike/type
        let call_qty = app.positions.iter()
            .find(|p| p.strike == item.strike_price && p.kind == crate::strategy::OptionType::Call)
            .map(|p| p.qty).unwrap_or(0);
        
        let put_qty = app.positions.iter()
            .find(|p| p.strike == item.strike_price && p.kind == crate::strategy::OptionType::Put)
            .map(|p| p.qty).unwrap_or(0);

        let call_text = if call_qty != 0 {
            format!("[{:+}] {:.2}", call_qty, call_ltp)
        } else {
            format!("{:.2}", call_ltp)
        };

        let put_text = if put_qty != 0 {
            format!("{:.2} [{:+}]", put_ltp, put_qty)
        } else {
            format!("{:.2}", put_ltp)
        };

        Row::new(vec![
            Cell::from(Line::from(call_text).alignment(Alignment::Right)).style(call_style),
            Cell::from(Line::from(format!("{:.0}", item.strike_price)).alignment(Alignment::Center)).style(strike_style),
            Cell::from(Line::from(put_text).alignment(Alignment::Left)).style(put_style),
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

    // --- STRATEGY PANEL ---
    if !app.positions.is_empty() {
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
            
            // text.push(Line::from(""));
            text.push(Line::from(Span::styled("Analysis:", Style::default().add_modifier(Modifier::UNDERLINED))));
            
            let max_profit_s = format!("{:.0}", stats.max_profit);
            let max_loss_s = format!("{:.0}", stats.max_loss);

            text.push(Line::from(vec![
                 Span::raw("Max Profit: "),
                 Span::styled(max_profit_s, Style::default().fg(Color::Green)),
            ]));
            text.push(Line::from(vec![
                 Span::raw("Max Loss:   "),
                 Span::styled(max_loss_s, Style::default().fg(Color::Red)),
            ]));

            if !stats.breakevens.is_empty() {
                 let be_str: Vec<String> = stats.breakevens.iter().map(|b| format!("{:.0}", b)).collect();
                 text.push(Line::from(vec![
                     Span::raw("Breakeven:  "),
                     Span::styled(be_str.join(", "), Style::default().fg(Color::Cyan)),
                 ]));
            }

            let block = Block::default().borders(Borders::ALL).title(" Analysis ");
            let paragraph = Paragraph::new(text).block(block);
            f.render_widget(paragraph, strategy_chunks[0]);

            // --- GRAPH ---
            let x_labels = vec![
                Span::styled(format!("{:.0}", stats.points.first().map(|p| p.0).unwrap_or(0.0)), Style::default().fg(Color::Gray)),
                Span::styled(format!("{:.0}", stats.points.last().map(|p| p.0).unwrap_or(0.0)), Style::default().fg(Color::Gray)),
            ];
            
            let y_min = stats.max_loss.min(0.0) * 1.1; // margin
            let y_max = stats.max_profit.max(0.0) * 1.1;
            
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
                    .bounds([stats.points.first().map(|p| p.0).unwrap_or(0.0), stats.points.last().map(|p| p.0).unwrap_or(100.0)])
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
}

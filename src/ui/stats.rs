use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use crate::app::App;
use crate::strategy::StrategyStats;

pub fn draw(f: &mut Frame, app: &App, stats: &StrategyStats, area: Rect) {
    // Helper for Indian Number System Formatting
    let format_indian = |val: f64| -> String {
        let abs_val = val.abs();
        let int_part = abs_val as u64;
        let s = int_part.to_string();
        let mut bytes = s.into_bytes();
        let len = bytes.len();
        
        let result = if len > 3 {
            let last_three = String::from_utf8(bytes.split_off(len - 3)).unwrap();
            let remaining = String::from_utf8(bytes).unwrap();
            
            let mut groups = Vec::new();
            let r_chars: Vec<char> = remaining.chars().rev().collect();
            for chunk in r_chars.chunks(2) {
                let g: String = chunk.iter().rev().collect();
                groups.push(g);
            }
            groups.reverse();
            
            groups.join(",") + "," + &last_three
        } else {
            String::from_utf8(bytes).unwrap()
        };
        
        let sign = if val < 0.0 { "-" } else { "" };
        format!("{}₹{}", sign, result)
    };

    let mut text = vec![
        Line::from(Span::styled(" Strategy Builder ", Style::default().add_modifier(Modifier::BOLD).bg(Color::Blue).fg(Color::White))),
        Line::from(""),
    ];

    // List Legs (Compact)
    text.push(Line::from(Span::styled("Active Legs:", Style::default().add_modifier(Modifier::UNDERLINED))));
    if app.portfolio.positions.is_empty() {
        text.push(Line::from(Span::styled(" No active positions.", Style::default().fg(Color::DarkGray))));
        text.push(Line::from(Span::styled(" Select strikes and press B/S to build.", Style::default().fg(Color::DarkGray))));
    }
    for pos in &app.portfolio.positions {
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
    if !app.portfolio.positions.is_empty() {
        // text.push(Line::from(""));
        text.push(Line::from(Span::styled("Analysis:", Style::default().add_modifier(Modifier::UNDERLINED))));
        
        let max_profit_s = if stats.max_profit_unlimited {
                Span::styled("Unlimited", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
        } else {
                Span::styled(format_indian(stats.max_profit), Style::default().fg(Color::Green))
        };

        let max_loss_s = if stats.max_loss_unlimited {
                Span::styled("Unlimited", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
        } else {
                Span::styled(format_indian(stats.max_loss), Style::default().fg(Color::Red))
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

        text.push(Line::from(vec![
            Span::raw("Prob. of Profit: "),
            Span::styled(format!("{:.1}%", stats.pop * 100.0), Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ]));
    }
    let paragraph = Paragraph::new(text).block(block);
    f.render_widget(paragraph, area);
}

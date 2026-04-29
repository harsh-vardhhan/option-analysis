use crate::app::App;
use crate::strategy::StrategyStats;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn draw(f: &mut Frame, app: &App, stats: &StrategyStats, area: Rect) {
    // Helper removed, using crate::ui::format_indian_currency

    let mut text = vec![
        Line::from(Span::styled(
            " Strategy Builder ",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .bg(Color::Blue)
                .fg(Color::White),
        )),
        Line::from(""),
    ];

    // List Legs (Compact)
    text.push(Line::from(Span::styled(
        "Active Legs:",
        Style::default().add_modifier(Modifier::UNDERLINED),
    )));
    if app.portfolio.positions.is_empty() {
        text.push(Line::from(Span::styled(
            " No active positions.",
            Style::default().fg(Color::DarkGray),
        )));
        text.push(Line::from(Span::styled(
            " Select strikes and press B/S to build.",
            Style::default().fg(Color::DarkGray),
        )));
    }
    for pos in &app.portfolio.positions {
        let side = if pos.qty > 0 { "BUY" } else { "SELL" };
        let color = if pos.qty > 0 {
            Color::Green
        } else {
            Color::Red
        };
        let kind = match pos.kind {
            crate::strategy::OptionType::Call => "CE",
            crate::strategy::OptionType::Put => "PE",
        };
        text.push(Line::from(vec![
            Span::styled(
                format!(" {:<4} ", side),
                Style::default().bg(color).fg(Color::Black),
            ),
            Span::raw(format!(
                " {} {} @ {:.1}",
                pos.qty.abs(),
                kind,
                pos.entry_price
            )),
            Span::styled(
                format!("  Str: {:.0}", pos.strike),
                Style::default().fg(Color::Yellow),
            ),
        ]));
    }

    let block = Block::default().borders(Borders::ALL).title(" Analysis ");
    if !app.portfolio.positions.is_empty() {
        // text.push(Line::from(""));
        text.push(Line::from(Span::styled(
            "Analysis:",
            Style::default().add_modifier(Modifier::UNDERLINED),
        )));

        let max_profit_s = if stats.max_profit_unlimited {
            Span::styled(
                "Unlimited",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Span::styled(
                crate::ui::format_indian_currency(stats.max_profit),
                Style::default().fg(Color::Green),
            )
        };

        let max_loss_s = if stats.max_loss_unlimited {
            Span::styled(
                "Unlimited",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )
        } else {
            Span::styled(
                crate::ui::format_indian_currency(stats.max_loss),
                Style::default().fg(Color::Red),
            )
        };

        text.push(Line::from(vec![Span::raw("Max Profit: "), max_profit_s]));
        text.push(Line::from(vec![Span::raw("Max Loss:   "), max_loss_s]));

        if !stats.breakevens.is_empty() {
            let be_str: Vec<String> = stats
                .breakevens
                .iter()
                .map(|b| format!("{:.0}", b))
                .collect();
            text.push(Line::from(vec![
                Span::raw("Breakeven:  "),
                Span::styled(be_str.join(", "), Style::default().fg(Color::Cyan)),
            ]));
        }

        text.push(Line::from(vec![
            Span::raw("Prob. of Profit: "),
            Span::styled(
                format!("{:.1}%", stats.pop * 100.0),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
    }
    let paragraph = Paragraph::new(text).block(block);
    f.render_widget(paragraph, area);
}

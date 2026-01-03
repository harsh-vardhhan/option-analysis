pub mod dashboard;
pub mod table;
pub mod stats;
pub mod chart;
pub mod help;
pub mod strategies;
pub mod setup;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    Frame,
};
use crate::app::App;

pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

/// Formats a number using the Indian Numbering System (e.g., 1,00,000)
pub fn format_indian_currency(val: f64) -> String {
    let abs_val = val.abs();
    let int_part = abs_val as u64;
    let s = int_part.to_string();
    
    let len = s.len();
    let result = if len > 3 {
        let (remaining, last_three) = s.split_at(len - 3);
        
        let mut groups = Vec::new();
        // Process remaining part in groups of 2 (reversed)
        // We use chars() to be safe, though these are digits.
        let r_chars: Vec<char> = remaining.chars().rev().collect();
        
        for chunk in r_chars.chunks(2) {
            let g: String = chunk.iter().rev().collect();
            groups.push(g);
        }
        groups.reverse();
        
        format!("{},{}", groups.join(","), last_three)
    } else {
        s
    };
    
    let sign = if val < 0.0 { "-" } else { "" };
    format!("{}₹{}", sign, result)
}

pub fn draw(f: &mut Frame, app: &mut App) {
    let constraints = vec![
        Constraint::Length(4), 
        Constraint::Min(0), 
        Constraint::Length(15)
    ];

    let chunks = Layout::default()
        .constraints(constraints)
        .split(f.size());

    // --- DASHBOARD ---
    dashboard::draw(f, app, chunks[0]);

    if app.data.is_empty() {
        return;
    }

    // --- MIDDLE SECTION (Table + Strategies) ---
    // Split chunks[1] into Table (80%) and Strategies (20%)
    let middle_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(80), Constraint::Percentage(20)])
        .split(chunks[1]);

    // --- TABLE ---
    table::draw(f, app, middle_chunks[0]);
    
    // --- STRATEGIES ---
    strategies::draw(f, app, middle_chunks[1]);

    // --- STRATEGY PANEL ---
    if chunks.len() > 2 {
        // Split the bottom chunk into Left (Stats) and Right (Graph)
        let strategy_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(chunks[2]);

use chrono::{NaiveDate, Local};

// ... 

        // Calculate Stats
        // Helper to get spot price for Strategy Analysis
        let spot_price = app.data.first().map(|d| d.underlying_spot_price).unwrap_or(0.0);
        
        // Calculate Days to Expiry
        let expiry_str = app.data.first().map(|d| d.expiry.as_str()).unwrap_or("");
        let days_to_expiry = if let Ok(expiry_date) = NaiveDate::parse_from_str(expiry_str, "%Y-%m-%d") {
            let today = Local::now().date_naive();
            (expiry_date - today).num_days().max(1) as f64 // at least 1 day to avoid div/0
        } else {
            1.0
        };

        // Find ATM IV (Average of Call/Put IV at closest strike)
        let atm_iv = if !app.data.is_empty() {
             let closest = app.data.iter().min_by(|a, b| {
                (a.strike_price - spot_price).abs().partial_cmp(&(b.strike_price - spot_price).abs()).unwrap_or(std::cmp::Ordering::Equal)
            });
            
            if let Some(row) = closest {
                let call_iv = row.call_options.as_ref().and_then(|o| o.option_greeks.as_ref()).map(|g| g.iv).unwrap_or(0.0);
                let put_iv = row.put_options.as_ref().and_then(|o| o.option_greeks.as_ref()).map(|g| g.iv).unwrap_or(0.0);
                if call_iv > 0.0 && put_iv > 0.0 {
                    (call_iv + put_iv) / 2.0
                } else {
                    if call_iv > 0.0 { call_iv } else { put_iv }
                }
            } else {
                0.0
            }
        } else {
            0.0
        };

        // Calculate Chain Step
        let chain_step = if app.data.len() > 1 {
            let mut strikes: Vec<f64> = app.data.iter().map(|d| d.strike_price).collect();
            strikes.sort_by(|a, b| a.partial_cmp(b).unwrap());
            strikes.dedup();
            
            let mut min_diff = f64::INFINITY;
            for window in strikes.windows(2) {
                let diff = (window[1] - window[0]).abs();
                if diff < min_diff && diff > 1.0 {
                    min_diff = diff;
                }
            }
            if min_diff != f64::INFINITY { min_diff } else { 50.0 }
        } else {
            50.0
        };

        use crate::strategy::analyze_strategy;
        let stats = analyze_strategy(&app.portfolio.positions, spot_price, atm_iv, days_to_expiry, chain_step);

        // Render Stats
        stats::draw(f, app, &stats, strategy_chunks[0]);
        
        // Render Chart
        chart::draw(f, app, &stats, strategy_chunks[1]);
    }

    // --- HELP OVERLAY ---
    help::draw(f, app);
}

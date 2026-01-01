pub mod dashboard;
pub mod table;
pub mod stats;
pub mod chart;
pub mod help;
pub mod strategies;

use ratatui::{
    layout::{Constraint, Direction, Layout},
    Frame,
};
use crate::app::App;

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
                (a.strike_price - spot_price).abs().partial_cmp(&(b.strike_price - spot_price).abs()).unwrap()
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

        use crate::strategy::analyze_strategy;
        let stats = analyze_strategy(&app.positions, spot_price, atm_iv, days_to_expiry);

        // Render Stats
        stats::draw(f, app, &stats, strategy_chunks[0]);
        
        // Render Chart
        chart::draw(f, app, &stats, strategy_chunks[1]);
    }

    // --- HELP OVERLAY ---
    help::draw(f, app);
}

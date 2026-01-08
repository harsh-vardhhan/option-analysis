use ratatui::layout::{Constraint, Direction, Layout, Rect};

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

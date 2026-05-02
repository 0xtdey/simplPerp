use crate::app::App;
use ratatui::{
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table},
    Frame,
};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default().borders(Borders::ALL).title("History");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let header = Row::new(vec!["Time", "Side", "Price", "Size", "PnL"])
        .style(Style::default().add_modifier(Modifier::BOLD));

    let mut rows = vec![];
    for fill in app.user.fill_history.iter().rev().take(50) {
        let pnl_color = if fill.pnl >= rust_decimal::Decimal::ZERO {
            Color::Green
        } else {
            Color::Red
        };
        rows.push(Row::new(vec![
            Cell::from(fill.timestamp.format("%H:%M:%S").to_string()),
            Cell::from(format!("{:?}", fill.side)),
            Cell::from(format!("{:.2}", fill.price)),
            Cell::from(format!("{:.4}", fill.size)),
            Cell::from(format!("{:.2}", fill.pnl)).style(Style::default().fg(pnl_color)),
        ]));
    }

    let table = Table::new(rows)
        .header(header)
        .widths(&[
            Constraint::Length(10),
            Constraint::Length(8),
            Constraint::Length(12),
            Constraint::Length(10),
            Constraint::Length(12),
        ]);
    f.render_widget(table, inner);
}

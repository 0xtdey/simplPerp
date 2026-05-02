use crate::app::App;
use crate::ui::theme;
use ratatui::{
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table},
    Frame,
};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::BORDER))
        .title(" Fill History ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let mut rows = Vec::new();
    for fill in app.user.fill_history.iter().rev().take(50) {
        let side_color = if fill.side == crate::engine::orderbook::OrderSide::Buy { theme::GREEN } else { theme::RED };
        rows.push(Row::new(vec![
            Cell::from(fill.timestamp.format("%H:%M:%S").to_string()).style(Style::default().fg(theme::TEXT_FAINT)),
            Cell::from(format!("{:?}", fill.side)).style(Style::default().fg(side_color)),
            Cell::from(format!("{:.2}", fill.price)).style(Style::default().fg(theme::TEXT_PRIMARY)),
            Cell::from(format!("{:.4}", fill.size)).style(Style::default().fg(theme::TEXT_PRIMARY)),
            Cell::from(format!("{:.2}", fill.pnl)).style(Style::default().fg(theme::TEXT_MUTED)),
        ]));
    }

    if rows.is_empty() {
        rows.push(Row::new(vec![
            Cell::from("No fills yet").style(Style::default().fg(theme::TEXT_MUTED)),
        ]));
    }

    let table = Table::new(rows)
        .header(
            Row::new(vec!["Time", "Side", "Price", "Size", "PnL"])
                .style(Style::default().fg(theme::TEXT_MUTED).add_modifier(Modifier::BOLD)),
        )
        .widths(&[
            Constraint::Percentage(18),
            Constraint::Percentage(12),
            Constraint::Percentage(22),
            Constraint::Percentage(22),
            Constraint::Percentage(26),
        ]);

    f.render_widget(table, inner);
}

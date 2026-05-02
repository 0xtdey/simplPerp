use crate::app::App;
use ratatui::{
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table},
    Frame,
};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let market = app.engine.current_market();
    let block = Block::default().borders(Borders::ALL).title(format!("Positions - {}", market.symbol));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let header = Row::new(vec!["Market", "Side", "Size", "Entry", "Mark", "PnL", "Margin", "Liq Price"])
        .style(Style::default().add_modifier(Modifier::BOLD));

    let mut rows = vec![];
    let mark = market.oracle.price();

    for (market_key, pos) in &app.user.positions {
        let pnl_color = if pos.unrealized_pnl >= rust_decimal::Decimal::ZERO {
            Color::Green
        } else {
            Color::Red
        };
        rows.push(Row::new(vec![
            Cell::from(market_key.clone()),
            Cell::from(format!("{:?}", pos.side)),
            Cell::from(format!("{:.4}", pos.size)),
            Cell::from(format!("{:.2}", pos.entry_price)),
            Cell::from(format!("{:.2}", mark)),
            Cell::from(format!("{:.2}", pos.unrealized_pnl)).style(Style::default().fg(pnl_color)),
            Cell::from(format!("{:.2}", pos.margin)),
            Cell::from(format!("{:.2}", pos.liquidation_price)).style(Style::default().fg(Color::Red)),
        ]));
    }

    let table = Table::new(rows)
        .header(header)
        .widths(&[
            Constraint::Length(12),
            Constraint::Length(8),
            Constraint::Length(10),
            Constraint::Length(12),
            Constraint::Length(12),
            Constraint::Length(12),
            Constraint::Length(10),
            Constraint::Length(12),
        ]);
    f.render_widget(table, inner);
}

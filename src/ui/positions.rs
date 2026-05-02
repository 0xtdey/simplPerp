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
        .title(" Positions ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let positions = &app.user.positions;
    let mut rows = Vec::new();

    for (_symbol, pos) in positions {
        let pnl = pos.unrealized_pnl;
        let pnl_color = if pnl >= rust_decimal::Decimal::ZERO { theme::GREEN } else { theme::RED };
        let pnl_sign = if pnl >= rust_decimal::Decimal::ZERO { "+" } else { "" };
        let side_color = if pos.side == crate::engine::orderbook::OrderSide::Buy { theme::GREEN } else { theme::RED };

        rows.push(Row::new(vec![
            Cell::from(format!("{:?}", pos.side)).style(Style::default().fg(side_color)),
            Cell::from(format!("{:.4}", pos.size)).style(Style::default().fg(theme::TEXT_PRIMARY)),
            Cell::from(format!("{:.2}", pos.entry_price)).style(Style::default().fg(theme::TEXT_PRIMARY)),
            Cell::from(format!("{:.2}", pos.leverage)).style(Style::default().fg(theme::TEXT_MUTED)),
            Cell::from(format!("{}{:.2}", pnl_sign, pnl)).style(Style::default().fg(pnl_color)),
            Cell::from(format!("{:.2}", pos.liquidation_price)).style(Style::default().fg(theme::RED)),
        ]));
    }

    if rows.is_empty() {
        rows.push(Row::new(vec![
            Cell::from("No open positions").style(Style::default().fg(theme::TEXT_MUTED)),
        ]));
    }

    let table = Table::new(rows)
        .header(
            Row::new(vec!["Side", "Size", "Entry", "Lev", "PnL", "Liq Price"])
                .style(Style::default().fg(theme::TEXT_MUTED).add_modifier(Modifier::BOLD)),
        )
        .widths(&[
            Constraint::Percentage(12),
            Constraint::Percentage(18),
            Constraint::Percentage(18),
            Constraint::Percentage(12),
            Constraint::Percentage(22),
            Constraint::Percentage(18),
        ]);

    f.render_widget(table, inner);
}

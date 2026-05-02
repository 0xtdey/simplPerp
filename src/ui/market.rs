use crate::app::App;
use crate::ui::chart::render_candlestick_chart;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Frame,
};

pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    render_orderbook(f, app, chunks[0]);
    render_chart_and_trades(f, app, chunks[1]);
}

fn render_orderbook(f: &mut Frame, app: &App, area: Rect) {
    let market = app.engine.current_market();
    let block = Block::default().borders(Borders::ALL).title(format!("Order Book ({})", market.symbol));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let (bids, asks) = market.orderbook.l2_snapshot(12);
    let mut rows = Vec::new();

    for (price, size) in asks.iter().rev() {
        rows.push(Row::new(vec![
            Cell::from(format!("{:.2}", price)).style(Style::default().fg(Color::Red)),
            Cell::from(format!("{:.4}", size)),
        ]));
    }

    let mark = market.oracle.price();
    rows.push(Row::new(vec![
        Cell::from(format!("{:.2}", mark)).style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Cell::from("MARK"),
    ]));

    for (price, size) in bids.iter() {
        rows.push(Row::new(vec![
            Cell::from(format!("{:.2}", price)).style(Style::default().fg(Color::Green)),
            Cell::from(format!("{:.4}", size)),
        ]));
    }

    let table = Table::new(rows)
        .header(Row::new(vec!["Price", "Size"]).style(Style::default().add_modifier(Modifier::BOLD)))
        .widths(&[Constraint::Percentage(50), Constraint::Percentage(50)]);
    f.render_widget(table, inner);
}

fn render_chart_and_trades(f: &mut Frame, app: &App, area: Rect) {
    let market = app.engine.current_market();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0), Constraint::Length(12)])
        .split(area);

    let price = market.oracle.price();
    let header = Paragraph::new(format!(
        "{}\nMark: {:.2} | Index: {:.2} | Press 't'=timeframe, 'm'=market",
        market.symbol, price, market.oracle.index_price()
    ))
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(header, chunks[0]);

    let candles = market.chart.candles();
    render_candlestick_chart(f, chunks[1], &candles, market.chart.timeframe, &market.symbol);

    let trades: Vec<_> = market.recent_trades.iter().rev().take(20).map(|t| {
        let color = match t.side {
            crate::engine::orderbook::OrderSide::Buy => Color::Green,
            crate::engine::orderbook::OrderSide::Sell => Color::Red,
        };
        Line::from(vec![
            Span::styled(format!("{:.2}", t.price), Style::default().fg(color)),
            Span::raw(" | "),
            Span::raw(format!("{:.4}", t.size)),
        ])
    }).collect();
    let trades_widget = Paragraph::new(trades)
        .block(Block::default().borders(Borders::ALL).title("Recent Trades"));
    f.render_widget(trades_widget, chunks[2]);
}

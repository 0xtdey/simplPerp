use crate::app::App;
use crate::engine::candles::Candle;
use crate::ui::chart::render_candlestick_chart;
use crate::ui::theme;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;

pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
    // Responsive split: wider terminals give more space to the chart
    let ob_pct = if area.width < 80 {
        35
    } else if area.width < 120 {
        30
    } else {
        25
    };
    let chart_pct = 100 - ob_pct;

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(ob_pct as u16), Constraint::Percentage(chart_pct as u16)])
        .split(area);

    render_orderbook(f, app, chunks[0]);
    render_market_right(f, app, chunks[1]);
}

fn render_orderbook(f: &mut Frame, app: &App, area: Rect) {
    let market = app.engine.current_market();
    let mark = market.oracle.price();
    let (best_bid, best_ask, spread) = market.spread().unwrap_or((Decimal::ZERO, Decimal::ZERO, Decimal::ZERO));

    let title = format!(
        " Order Book  B:{:.1} A:{:.1} ",
        best_bid, best_ask
    );
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::BORDER))
        .title(title);
    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.width < 12 || inner.height < 10 {
        return;
    }

    // Adaptive snapshot depth
    let depth = if inner.height > 40 { 20 } else if inner.height > 25 { 14 } else { 8 };
    let (bids, asks) = market.orderbook.l2_snapshot(depth);

    // Adaptive column widths
    let price_w: usize = if inner.width < 25 { 8 } else { 9 };
    let qty_w: usize = 7;
    let total_w: usize = if inner.width < 30 { 6 } else { 7 };
    let depth_w = inner.width.saturating_sub((price_w + qty_w + total_w) as u16);

    // Header
    let header = if depth_w < 3 {
        // Compact header: skip total and depth when very narrow
        Line::from(vec![
            Span::styled(format!("{:<price_w$}", "Price", price_w = price_w), Style::default().fg(theme::TEXT_MUTED).add_modifier(Modifier::BOLD)),
            Span::styled(format!("{:<qty_w$}", "Qty", qty_w = qty_w), Style::default().fg(theme::TEXT_MUTED).add_modifier(Modifier::BOLD)),
        ])
    } else {
        Line::from(vec![
            Span::styled(format!("{:<price_w$}", "Price", price_w = price_w), Style::default().fg(theme::TEXT_MUTED).add_modifier(Modifier::BOLD)),
            Span::styled(format!("{:<qty_w$}", "Qty", qty_w = qty_w), Style::default().fg(theme::TEXT_MUTED).add_modifier(Modifier::BOLD)),
            Span::styled(format!("{:<total_w$}", "Total", total_w = total_w), Style::default().fg(theme::TEXT_MUTED).add_modifier(Modifier::BOLD)),
            Span::styled("Depth", Style::default().fg(theme::TEXT_MUTED).add_modifier(Modifier::BOLD)),
        ])
    };

    let show_depth = depth_w >= 3;

    // Compute cumulative volumes and find max for depth scaling
    let asks_asc: Vec<_> = asks.iter().rev().collect();
    let mut ask_cum: Vec<(Decimal, Decimal, Decimal)> = Vec::new();
    let mut cum = Decimal::ZERO;
    for (price, size) in &asks_asc {
        cum += size;
        ask_cum.push((*price, *size, cum));
    }

    let bids_desc: Vec<_> = bids.iter().collect();
    let mut bid_cum: Vec<(Decimal, Decimal, Decimal)> = Vec::new();
    cum = Decimal::ZERO;
    for (price, size) in &bids_desc {
        cum += size;
        bid_cum.push((*price, *size, cum));
    }

    let max_cum = ask_cum.last().map(|(_, _, c)| *c)
        .unwrap_or(Decimal::ZERO)
        .max(bid_cum.last().map(|(_, _, c)| *c).unwrap_or(Decimal::ZERO));
    let max_cum = max_cum.max(Decimal::ONE);

    let available_rows = inner.height.saturating_sub(2) as usize;
    let half = available_rows / 2;
    // Subtract 1 for the spread row
    let ask_rows = (ask_cum.len().min(half.saturating_sub(1))).min(available_rows.saturating_sub(2));
    let bid_rows = (bid_cum.len().min(half.saturating_sub(1))).min(available_rows.saturating_sub(2));

    let mut lines: Vec<Line> = Vec::with_capacity(available_rows + 2);
    lines.push(header);

    // Asks (closest to mark at bottom)
    let ask_display: Vec<_> = ask_cum.iter().take(ask_rows).collect();
    for (price, qty, cum) in ask_display.iter().rev() {
        let price_f = price.to_f64().unwrap_or(0.0);
        let qty_f = qty.to_f64().unwrap_or(0.0);
        let cum_f = cum.to_f64().unwrap_or(0.0);

        let mut spans = Vec::new();
        spans.push(Span::styled(
            format!("{:<price_w$.2}", price_f, price_w = price_w),
            Style::default().fg(theme::RED),
        ));
        spans.push(Span::styled(
            format!("{:<qty_w$.3}", qty_f, qty_w = qty_w),
            Style::default().fg(theme::TEXT_PRIMARY),
        ));

        if show_depth {
            spans.push(Span::styled(
                format!("{:<total_w$.3}", cum_f, total_w = total_w),
                Style::default().fg(theme::TEXT_MUTED),
            ));
            let bar_width = ((*cum / max_cum).to_f64().unwrap_or(0.0) * depth_w as f64) as usize;
            let bar = "█".repeat(bar_width.min(depth_w as usize));
            spans.push(Span::styled(bar, Style::default().fg(theme::RED_DEPTH)));
        }

        lines.push(Line::from(spans));
    }

    // Spread row
    {
        let spread_str = if spread > Decimal::ZERO {
            format!("── SPREAD: {:.2} ──", spread)
        } else {
            "── SPREAD: -- ──".to_string()
        };
        let total_line_width = inner.width as usize;
        let pad = total_line_width.saturating_sub(spread_str.len()) / 2;
        lines.push(Line::from(vec![
            Span::styled(
                format!("{}{}", " ".repeat(pad), spread_str),
                Style::default().fg(theme::AMBER).add_modifier(Modifier::BOLD),
            ),
        ]));
    }

    // Bids (closest to mark at top)
    for (price, qty, cum) in bid_cum.iter().take(bid_rows) {
        let price_f = price.to_f64().unwrap_or(0.0);
        let qty_f = qty.to_f64().unwrap_or(0.0);
        let cum_f = cum.to_f64().unwrap_or(0.0);

        let mut spans = Vec::new();
        spans.push(Span::styled(
            format!("{:<price_w$.2}", price_f, price_w = price_w),
            Style::default().fg(theme::GREEN),
        ));
        spans.push(Span::styled(
            format!("{:<qty_w$.3}", qty_f, qty_w = qty_w),
            Style::default().fg(theme::TEXT_PRIMARY),
        ));

        if show_depth {
            spans.push(Span::styled(
                format!("{:<total_w$.3}", cum_f, total_w = total_w),
                Style::default().fg(theme::TEXT_MUTED),
            ));
            let bar_width = ((*cum / max_cum).to_f64().unwrap_or(0.0) * depth_w as f64) as usize;
            let bar = "█".repeat(bar_width.min(depth_w as usize));
            spans.push(Span::styled(bar, Style::default().fg(theme::GREEN_DEPTH)));
        }

        lines.push(Line::from(spans));
    }

    // Mark price row at bottom if space
    if lines.len() < available_rows as usize + 1 {
        let mark_str = format!("› M: {:.2}", mark);
        lines.push(Line::from(vec![
            Span::styled(mark_str, Style::default().fg(theme::AMBER).add_modifier(Modifier::BOLD)),
        ]));
    }

    let ob = Paragraph::new(lines);
    f.render_widget(ob, inner);
}

fn render_market_right(f: &mut Frame, app: &mut App, area: Rect) {
    let market = app.engine.current_market();
    let stats = &market.stats_24h;
    let mark = market.oracle.price();
    let index = market.oracle.index_price();
    let funding = app.engine.funding.rate;
    let (_best_bid, _best_ask, spread) = market.spread().unwrap_or((Decimal::ZERO, Decimal::ZERO, Decimal::ZERO));
    let change = stats.change_pct();
    let candles = market.chart.candles();
    let last_c: Option<&&Candle> = candles.last();

    let stats_rows: u16 = if area.height < 30 { 2 } else if area.height < 40 { 3 } else { 5 };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(stats_rows),
            Constraint::Min(5),
            Constraint::Length(1),
        ])
        .split(area);

    render_stats_bar(f, app, chunks[0], mark, index, funding, spread, change, last_c);

    render_candlestick_chart(
        f, chunks[1], &candles, market.chart.timeframe, &market.symbol,
        &market.fill_markers,
        if app.crosshair_enabled && app.current_screen == crate::app::Screen::Market {
            Some(app.crosshair_col)
        } else {
            None
        },
        &mut app.crosshair_info,
    );

    render_recent_trades(f, app, chunks[2]);
}

fn render_stats_bar(
    f: &mut Frame,
    app: &App,
    area: Rect,
    mark: Decimal,
    index: Decimal,
    funding: Decimal,
    spread: Decimal,
    change: Decimal,
    last_candle: Option<&&Candle>,
) {
    let market = app.engine.current_market();
    let stats = &market.stats_24h;

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::BORDER))
        .title(format!(" {} ", market.symbol));
    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.height < 1 {
        return;
    }

    let change_color = if change >= Decimal::ZERO { theme::GREEN } else { theme::RED };
    let change_sign = if change >= Decimal::ZERO { "+" } else { "" };
    let price_color = if stats.prev_close >= stats.open_price { theme::GREEN } else { theme::RED };

    let mut rows: Vec<Line> = Vec::new();

    // Row 1: Mark Price + 24h Change (always show)
    rows.push(Line::from(vec![
        Span::styled(
            format!("  {:.2}", mark),
            Style::default().fg(price_color).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("  {}{:.2}%", change_sign, change),
            Style::default().fg(change_color).add_modifier(Modifier::BOLD),
        ),
    ]));

    // If terminal is tall enough (inner.height >= 2), add more rows
    if inner.height >= 2 {
        // Row 2: 24h H/L + Volume
        rows.push(Line::from(vec![
            Span::styled(
                format!("  H:{:.1} L:{:.1}", stats.high, stats.low),
                Style::default().fg(theme::TEXT_PRIMARY),
            ),
            Span::styled(
                format!("  Vol:{:.2}", stats.volume),
                Style::default().fg(theme::TEXT_MUTED),
            ),
        ]));
    }

    // Row 3: Funding, Spread (when 3+ rows available)
    if inner.height >= 3 {
        let fund_color = if funding >= Decimal::ZERO { theme::GREEN } else { theme::RED };
        let fund_sign = if funding >= Decimal::ZERO { "+" } else { "" };
        rows.push(Line::from(vec![
            Span::styled(
                format!("  Fund:{}{:.4}%", fund_sign, funding * Decimal::from(100)),
                Style::default().fg(fund_color),
            ),
            Span::styled(
                format!("  Sprd:{:.2}", spread),
                Style::default().fg(theme::AMBER),
            ),
            Span::styled(
                format!("  M:{:.2} I:{:.2}", mark, index),
                Style::default().fg(theme::TEXT_MUTED),
            ),
        ]));
    } else if inner.height >= 2 {
        // Compact funding line for medium height
        let fund_color = if funding >= Decimal::ZERO { theme::GREEN } else { theme::RED };
        let fund_sign = if funding >= Decimal::ZERO { "+" } else { "" };
        rows.push(Line::from(vec![
            Span::styled(
                format!("  Fund:{}{:.4}%  Sprd:{:.2}", fund_sign, funding * Decimal::from(100), spread),
                Style::default().fg(fund_color),
            ),
        ]));
    }

    // Row 4: OHLCV of current candle (when 4+ rows available)
    if inner.height >= 4 {
        if let Some(c) = last_candle {
            rows.push(Line::from(vec![
                Span::styled(format!("  O:{:.2}", c.open), Style::default().fg(theme::TEXT_PRIMARY)),
                Span::styled(format!(" H:{:.2}", c.high), Style::default().fg(theme::GREEN)),
                Span::styled(format!(" L:{:.2}", c.low), Style::default().fg(theme::RED)),
                Span::styled(format!(" C:{:.2}", c.close), Style::default().fg(theme::TEXT_PRIMARY)),
                Span::styled(format!(" V:{:.2}", c.volume), Style::default().fg(theme::TEXT_MUTED)),
            ]));
        }
    }

    f.render_widget(Paragraph::new(rows), inner);
}

fn render_recent_trades(f: &mut Frame, app: &App, area: Rect) {
    let market = app.engine.current_market();
    let max_trades = if area.width > 80 { 30 } else { 15 };
    let trades: Vec<_> = market.recent_trades.iter().rev().take(max_trades).collect();

    if trades.is_empty() {
        return;
    }

    let max_width = area.width as usize;
    let mut spans: Vec<Span> = Vec::new();
    let mut line_width = 0usize;

    for t in &trades {
        let side_ch = if t.side == crate::engine::orderbook::OrderSide::Buy { "B" } else { "S" };
        let color = if t.side == crate::engine::orderbook::OrderSide::Buy { theme::GREEN } else { theme::RED };
        let text = format!(" {:.2}{}{:.3} ", t.price, side_ch, t.size);
        let text_len = text.len();

        if line_width + text_len + 1 > max_width {
            break;
        }

        spans.push(Span::styled(text, Style::default().fg(color)));
        spans.push(Span::styled("│", Style::default().fg(theme::BORDER_DIM)));
        line_width += text_len + 1;
    }

    if spans.len() >= 2 {
        spans.pop();
    }

    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(theme::BORDER_DIM));
    f.render_widget(Paragraph::new(Line::from(spans)).block(block), area);
}

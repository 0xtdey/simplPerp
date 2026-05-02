use crate::engine::candles::{Candle, Timeframe};
use crate::engine::market::FillMarker;
use crate::engine::orderbook::OrderSide;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use std::cmp;

const TARGET_VISUAL_CANDLES: usize = 22;
const CANDLE_WIDTH: usize = 2;
const CANDLE_GAP: usize = 1;
const SLOT_WIDTH: usize = CANDLE_WIDTH + CANDLE_GAP;

pub fn render_candlestick_chart(
    f: &mut Frame,
    area: Rect,
    candles: &[&Candle],
    timeframe: Timeframe,
    symbol: &str,
    fill_markers: &[FillMarker],
    mut crosshair_col: Option<usize>,
    crosshair_info: &mut String,
) {
    crosshair_info.clear();

    if candles.is_empty() {
        let empty = Paragraph::new("Collecting data...")
            .block(Block::default().borders(Borders::ALL)
                .title(format!("{} | {}", symbol, timeframe.label())));
        f.render_widget(empty, area);
        return;
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!("{} | {}", symbol, timeframe.label()));
    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.height < 10 || inner.width < 20 {
        return;
    }

    let h_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(10), Constraint::Min(0)])
        .split(inner);

    let chart_area = h_chunks[1];
    let y_axis_area = h_chunks[0];

    // Compute price range
    let mut min_price = candles[0].low;
    let mut max_price = candles[0].high;
    let mut max_volume = Decimal::ZERO;
    for c in candles {
        if c.low < min_price { min_price = c.low; }
        if c.high > max_price { max_price = c.high; }
        if c.volume > max_volume { max_volume = c.volume; }
    }

    let range = max_price - min_price;
    let pad = if range == Decimal::ZERO { Decimal::from(1) } else { range / Decimal::from(20) };
    min_price -= pad;
    max_price += pad;
    let price_range = max_price - min_price;

    let vol_rows = 3usize;
    let sep_rows = 1usize;
    let chart_rows = (inner.height as usize).saturating_sub(vol_rows + sep_rows);
    if chart_rows == 0 { return; }

    // Build aggregated visual candles
    let visual_candles = build_visual_candles(candles, chart_area.width as usize);

    let max_slots = (chart_area.width as usize) / SLOT_WIDTH;
    let visible_visuals = visual_candles.len().min(max_slots);
    let start_idx = visual_candles.len().saturating_sub(visible_visuals);
    let visible: Vec<_> = visual_candles[start_idx..].to_vec();

    let cols = visible.len() * SLOT_WIDTH;

    // Clamp crosshair column
    if let Some(ref mut cc) = crosshair_col {
        *cc = (*cc).min(visible.len().saturating_sub(1));
    }

    // Initialize grid
    let mut grid: Vec<Vec<(char, Color)>> = vec![vec![(' ', Color::Gray); cols]; chart_rows];

    // Grid dots every 4 rows
    for row in (0..chart_rows).step_by(4) {
        for col in 0..cols {
            if grid[row][col].0 == ' ' {
                grid[row][col] = ('·', Color::DarkGray);
            }
        }
    }

    // Render candles
    for (i, vc) in visible.iter().enumerate() {
        let col_start = i * SLOT_WIDTH;
        let top_row = price_to_row(vc.high, min_price, price_range, chart_rows);
        let bot_row = price_to_row(vc.low, min_price, price_range, chart_rows);
        let body_top = price_to_row(vc.open.max(vc.close), min_price, price_range, chart_rows);
        let body_bot = price_to_row(vc.open.min(vc.close), min_price, price_range, chart_rows);

        let color = if vc.close >= vc.open { Color::Green } else { Color::Red };

        for row in bot_row..=top_row {
            if row >= chart_rows { continue; }
            let in_body = row >= body_bot && row <= body_top;
            let ch = if in_body { '█' } else { '│' };
            for w in 0..CANDLE_WIDTH {
                if col_start + w < cols {
                    grid[row][col_start + w] = (ch, color);
                }
            }
        }
    }

    // Render fill markers (▲ buy, ▼ sell)
    for fm in fill_markers {
        // Estimate which visual candle this fill belongs to
        // based on timestamp proximity
        let mut best_idx = None;
        for (i, vc) in visible.iter().enumerate() {
            let col_start = i * SLOT_WIDTH;
            if col_start + CANDLE_WIDTH > cols { break; }
            // Approximate: check if fill price is within candle range
            let marker_row = price_to_row(fm.price, min_price, price_range, chart_rows);
            if marker_row < chart_rows && fm.price >= vc.low && fm.price <= vc.high {
                best_idx = Some(i);
                break;
            }
        }

        if let Some(idx) = best_idx {
            let marker_row = price_to_row(fm.price, min_price, price_range, chart_rows);
            let col_start = idx * SLOT_WIDTH;

            if marker_row < chart_rows && col_start + CANDLE_WIDTH <= cols {
                let marker_color = match fm.side {
                    OrderSide::Buy => Color::Green,
                    OrderSide::Sell => Color::Red,
                };
                let marker_ch = match fm.side {
                    OrderSide::Buy => '▲',
                    OrderSide::Sell => '▼',
                };
                // Place marker above or below the candle
                let label_row = if marker_row > 0 { marker_row - 1 } else { marker_row };
                if label_row < chart_rows {
                    for w in 0..CANDLE_WIDTH {
                        grid[label_row][col_start + w] = (marker_ch, marker_color);
                    }
                }
            }
        }
    }

    // Crosshair cursor
    if let Some(cc) = crosshair_col {
        if cc < visible.len() {
            let vc = &visible[cc];
            let col = cc * SLOT_WIDTH;

            // Draw vertical cursor line on all rows
            for row in 0..chart_rows {
                if col < cols {
                    grid[row][col] = ('│', Color::White);
                    if col + 1 < cols {
                        grid[row][col + 1] = ('│', Color::White);
                    }
                }
            }

            *crosshair_info = format!(
                "Candle: O={:.1} H={:.1} L={:.1} C={:.1} | Vol={:.1}",
                vc.open.to_f64().unwrap_or(0.0),
                vc.high.to_f64().unwrap_or(0.0),
                vc.low.to_f64().unwrap_or(0.0),
                vc.close.to_f64().unwrap_or(0.0),
                vc.volume.to_f64().unwrap_or(0.0),
            );
        }
    }

    // Build lines
    let mut all_lines: Vec<Line> = Vec::with_capacity(chart_rows + sep_rows + vol_rows);

    for row in 0..chart_rows {
        let spans: Vec<Span> = grid[row]
            .iter()
            .map(|(ch, color)| Span::styled(ch.to_string(), Style::default().fg(*color)))
            .collect();
        all_lines.push(Line::from(spans));
    }

    // Separator
    let sep_spans: Vec<Span> = (0..cols)
        .map(|_| Span::styled("─", Style::default().fg(Color::DarkGray)))
        .collect();
    all_lines.push(Line::from(sep_spans));

    // Volume bars
    let mut vol_grid: Vec<Vec<(char, Color)>> = vec![vec![(' ', Color::Gray); cols]; vol_rows];
    for (i, vc) in visible.iter().enumerate() {
        let col_start = i * SLOT_WIDTH;
        if max_volume > Decimal::ZERO {
            let ratio = vc.volume / max_volume;
            let height = (ratio * Decimal::from(vol_rows))
                .to_f64().unwrap_or(0.0)
                .round() as usize;
            let color = if vc.close >= vc.open { Color::Green } else { Color::Red };
            for r in 0..height.min(vol_rows) {
                let row = vol_rows - 1 - r;
                for w in 0..CANDLE_WIDTH {
                    if col_start + w < cols {
                        vol_grid[row][col_start + w] = ('█', color);
                    }
                }
            }
        }
    }

    for row in 0..vol_rows {
        let spans: Vec<Span> = vol_grid[row]
            .iter()
            .map(|(ch, color)| Span::styled(ch.to_string(), Style::default().fg(*color)))
            .collect();
        all_lines.push(Line::from(spans));
    }

    // Y-axis labels
    let mut y_labels: Vec<Line> = Vec::with_capacity(chart_rows + sep_rows + vol_rows);
    for row in 0..chart_rows {
        let price = row_to_price(row, min_price, price_range, chart_rows);
        let label = 
            if row == 0 || row == chart_rows - 1 {
                format_price(price)
            } else if row == chart_rows / 2 {
                format_price(price)
            } else if chart_rows > 16 && (row == chart_rows / 4 || row == 3 * chart_rows / 4) {
                format_price(price)
            } else {
                String::new()
            };
        y_labels.push(Line::from(vec![
            Span::styled(format!("{:>8}", label), Style::default().fg(Color::DarkGray)),
            Span::styled("┤", Style::default().fg(Color::DarkGray)),
        ]));
    }
    y_labels.push(Line::from(Span::styled("       ┤", Style::default().fg(Color::DarkGray))));
    y_labels.push(Line::from(Span::styled("    VOL┤", Style::default().fg(Color::DarkGray))));
    y_labels.push(Line::from(Span::styled("       ┤", Style::default().fg(Color::DarkGray))));
    y_labels.push(Line::from(Span::styled("       ┤", Style::default().fg(Color::DarkGray))));

    let y_axis = Paragraph::new(y_labels);
    f.render_widget(y_axis, y_axis_area);

    let chart = Paragraph::new(all_lines);
    f.render_widget(chart, chart_area);
}

fn build_visual_candles(candles: &[&Candle], chart_width: usize) -> Vec<VisualCandle> {
    let max_slots = chart_width / SLOT_WIDTH;
    let target = cmp::min(TARGET_VISUAL_CANDLES, max_slots.max(1));

    if candles.len() <= target {
        return candles.iter().map(|c| VisualCandle {
            open: c.open, high: c.high, low: c.low, close: c.close, volume: c.volume,
        }).collect();
    }

    let n = candles.len() / target;
    let mut result = Vec::with_capacity(target);
    let mut i = 0;
    while i < candles.len() {
        let end = cmp::min(i + n, candles.len());
        let group = &candles[i..end];
        let open = group[0].open;
        let close = group[group.len() - 1].close;
        let high = group.iter().map(|c| c.high).fold(Decimal::ZERO, |a, b| a.max(b));
        let low = group.iter().map(|c| c.low).fold(Decimal::MAX, |a, b| a.min(b));
        let volume = group.iter().map(|c| c.volume).fold(Decimal::ZERO, |a, b| a + b);
        result.push(VisualCandle { open, high, low, close, volume });
        i = end;
    }
    result
}

#[derive(Clone, Copy)]
struct VisualCandle {
    open: Decimal,
    high: Decimal,
    low: Decimal,
    close: Decimal,
    volume: Decimal,
}

fn price_to_row(price: Decimal, min: Decimal, range: Decimal, rows: usize) -> usize {
    if range == Decimal::ZERO || rows <= 1 {
        return rows.saturating_sub(1);
    }
    let ratio = (price - min) / range;
    let row_f = ratio.to_f64().unwrap_or(0.0).clamp(0.0, 1.0);
    let row = (row_f * (rows - 1) as f64).round() as usize;
    (rows - 1).saturating_sub(row)
}

fn row_to_price(row: usize, min: Decimal, range: Decimal, rows: usize) -> Decimal {
    if rows <= 1 { return min; }
    let ratio = Decimal::from_f64_retain(row as f64 / (rows - 1) as f64).unwrap_or(Decimal::ZERO);
    min + range * ratio
}

fn format_price(price: Decimal) -> String {
    let f = price.to_f64().unwrap_or(0.0);
    if f >= 10_000.0 {
        format!("{:.0}", f)
    } else if f >= 100.0 {
        format!("{:.1}", f)
    } else {
        format!("{:.2}", f)
    }
}

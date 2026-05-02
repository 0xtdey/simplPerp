use crate::engine::candles::{Candle, Timeframe};
use crate::ui::theme;
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

const CANDLE_WIDTH: usize = 2;
const CANDLE_GAP: usize = 1;
const SLOT_WIDTH: usize = CANDLE_WIDTH + CANDLE_GAP;

/// A cell in the chart grid with a character, color, and z-layer priority.
/// Higher priority layers render on top of lower ones.
#[derive(Clone, Copy)]
struct GridCell {
    ch: char,
    color: Color,
    priority: u8,
}

impl GridCell {
    fn empty() -> Self {
        Self { ch: ' ', color: theme::BG, priority: 0 }
    }
}

pub fn render_candlestick_chart(
    f: &mut Frame,
    area: Rect,
    candles: &[&Candle],
    timeframe: Timeframe,
    symbol: &str,
    ema: &[Decimal],
    mark_price: Decimal,
) {
    if candles.is_empty() {
        let empty = Paragraph::new("Collecting data...")
            .block(Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme::BORDER))
                .title(format!("{} | {}", symbol, timeframe.label())));
        f.render_widget(empty, area);
        return;
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::BORDER))
        .title(format!("{} | {}", symbol, timeframe.label()));
    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.height < 7 || inner.width < 30 {
        return;
    }

    let y_label_width = if inner.width < 50 { 8usize } else { 10usize };
    let h_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(y_label_width as u16), Constraint::Min(0)])
        .split(inner);

    let chart_area = h_chunks[1];
    let y_axis_area = h_chunks[0];

    // Price range
    let mut min_price = candles[0].low;
    let mut max_price = candles[0].high;
    let mut max_volume = Decimal::ZERO;
    for c in candles {
        if c.low < min_price { min_price = c.low; }
        if c.high > max_price { max_price = c.high; }
        if c.volume > max_volume { max_volume = c.volume; }
    }
    // Include EMA and mark in range
    for &v in ema {
        if v > Decimal::ZERO && v < min_price { min_price = v; }
        if v > max_price { max_price = v; }
    }
    if mark_price < min_price { min_price = mark_price; }
    if mark_price > max_price { max_price = mark_price; }

    let range = max_price - min_price;
    let pad = if range == Decimal::ZERO { Decimal::from(1) } else { range / Decimal::from(20) };
    min_price -= pad;
    max_price += pad;
    let price_range = max_price - min_price;

    // Layout: chart | separator | volume | time_axis
    // Adaptive volume rows based on total inner height
    let total_inner = inner.height as usize;
    let vol_rows = if total_inner > 50 { 5 } else if total_inner > 35 { 4 } else if total_inner > 25 { 3 } else { 2 };
    let time_rows = 1usize;
    let sep_rows = 1usize;
    let chart_rows = (inner.height as usize).saturating_sub(vol_rows + sep_rows + time_rows);

    if chart_rows < 5 {
        return;
    }

    let effective_levels = chart_rows * 2;

    // Build visual candles — fill the full chart width
    let max_slots = (chart_area.width as usize) / SLOT_WIDTH;
    let visual_candles = build_visual_candles(candles, max_slots);
    let visible_visuals = visual_candles.len().min(max_slots);
    let start_idx = visual_candles.len().saturating_sub(visible_visuals);
    let visible: Vec<_> = visual_candles[start_idx..].to_vec();

    // Use the full chart area width, not just the candle slots
    let cols = chart_area.width as usize;

    // Dynamically scale candle positions to fill the full width
    let real_slot = if visible.len() > 1 {
        cols / visible.len()
    } else {
        cols
    };
    let body_width = if real_slot > CANDLE_GAP { real_slot - CANDLE_GAP } else { 1 };

    // Initialize grid
    let mut grid: Vec<Vec<GridCell>> = vec![vec![GridCell::empty(); cols]; chart_rows];

    // Layer 1: Grid dots — spacing adapts to chart height
    let grid_step = if chart_rows < 12 { 2 } else { 4 };
    let grid_step_bright = if chart_rows < 12 { 4 } else { 8 };

    for eff_level in (0..effective_levels).step_by(grid_step) {
        let row = eff_level / 2;
        if row < chart_rows {
            for col in 0..cols {
                grid[row][col] = GridCell { ch: '·', color: theme::BORDER_DIM, priority: 1 };
            }
        }
        // Brighter dot at double interval
        if eff_level % grid_step_bright == grid_step {
            let row = eff_level / 2;
            if row < chart_rows {
                for col in 0..cols {
                    grid[row][col] = GridCell { ch: '·', color: theme::TEXT_FAINT, priority: 2 };
                }
            }
        }
    }

    // Layer 10: Candle bodies + wicks (half-block precision)
    for (i, vc) in visible.iter().enumerate() {
        let col_start = i * real_slot;
        let color = if vc.close >= vc.open { theme::GREEN } else { theme::RED };

        let top_eff = price_to_eff(vc.high, min_price, price_range, effective_levels);
        let bot_eff = price_to_eff(vc.low, min_price, price_range, effective_levels);
        let body_top_eff = price_to_eff(vc.open.max(vc.close), min_price, price_range, effective_levels);
        let body_bot_eff = price_to_eff(vc.open.min(vc.close), min_price, price_range, effective_levels);

        // Render from low to high
        for eff in bot_eff..=top_eff {
            let row = eff / 2;
            let sub = eff % 2;
            if row >= chart_rows { break; }

            let in_body = eff >= body_bot_eff && eff <= body_top_eff;
            let is_top = eff == top_eff;
            let is_bot = eff == bot_eff;

            let ch = match (in_body, is_top, is_bot, sub) {
                (true, _, _, _) => {
                    let fills_both = body_bot_eff / 2 < body_top_eff / 2;
                    if !fills_both && body_top_eff / 2 == body_bot_eff / 2 {
                        if body_top_eff % 2 == 0 && body_bot_eff % 2 == 1 {
                            '█'
                        } else if body_top_eff % 2 == 0 && body_bot_eff % 2 == 0 {
                            '▀'
                        } else {
                            '▄'
                        }
                    } else {
                        '█'
                    }
                }
                (false, true, _, 0) => '╵',
                (false, true, _, 1) => '│',
                (false, _, true, 0) => '│',
                (false, _, true, 1) => '╷',
                _ => '│',
            };

            for w in 0..body_width {
                if col_start + w < cols {
                    grid[row][col_start + w] = GridCell { ch, color, priority: 10 };
                }
            }
        }
    }

    // Layer 15: EMA line (blue dots)
    if ema.len() >= visible.len() {
        let ema_start = ema.len().saturating_sub(visible_visuals);
        let ema_slice = &ema[ema_start..];
        for (i, &ema_val) in ema_slice.iter().enumerate() {
            if ema_val == Decimal::ZERO || i >= visible.len() { continue; }
            let col = i * real_slot;
            let eff = price_to_eff(ema_val, min_price, price_range, effective_levels);
            let row = (eff / 2).min(chart_rows.saturating_sub(1));
            if row < chart_rows {
                for w in 0..body_width {
                    if col + w < cols {
                        if grid[row][col + w].priority < 15 {
                            grid[row][col + w] = GridCell { ch: '●', color: theme::BLUE, priority: 15 };
                        }
                    }
                }
            }
        }
    }

    // Layer 20: Mark price line (amber dashes)
    let mark_eff = price_to_eff(mark_price, min_price, price_range, effective_levels);
    let mark_row = (mark_eff / 2).min(chart_rows.saturating_sub(1));
    if mark_row < chart_rows {
        for col in 0..cols {
            // Don't overwrite candle body/wicks, but overwrite grid dots
            if grid[mark_row][col].priority < 10 {
                grid[mark_row][col] = GridCell {
                    ch: if col % 2 == 0 { '─' } else { '─' },
                    color: theme::AMBER,
                    priority: 20,
                };
            }
        }
    }

    // Build chart lines
    let mut all_lines: Vec<Line> = Vec::with_capacity(chart_rows + sep_rows + vol_rows + time_rows);

    for row in 0..chart_rows {
        let spans: Vec<Span> = grid[row]
            .iter()
            .map(|cell| Span::styled(cell.ch.to_string(), Style::default().fg(cell.color).bg(theme::BG)))
            .collect();
        all_lines.push(Line::from(spans));
    }

    // Separator row
    let sep_spans: Vec<Span> = (0..cols)
        .map(|_| Span::styled("─", Style::default().fg(theme::BORDER_DIM)))
        .collect();
    all_lines.push(Line::from(sep_spans));

    // Volume bars
    let mut vol_grid: Vec<Vec<GridCell>> = vec![vec![GridCell::empty(); cols]; vol_rows];
    for (i, vc) in visible.iter().enumerate() {
        let col_start = i * real_slot;
        if max_volume > Decimal::ZERO {
            let ratio = vc.volume / max_volume;
            let height_f = ratio.to_f64().unwrap_or(0.0) * (vol_rows as f64);
            let height = height_f.ceil() as usize;
            let color = if vc.close >= vc.open { theme::GREEN } else { theme::RED };

            for r in 0..height.min(vol_rows) {
                let row = vol_rows - 1 - r;
                for w in 0..body_width {
                    if col_start + w < cols {
                        vol_grid[row][col_start + w] = GridCell { ch: '█', color, priority: 10 };
                    }
                }
            }
        }
    }
    for row in 0..vol_rows {
        let spans: Vec<Span> = vol_grid[row]
            .iter()
            .map(|cell| Span::styled(cell.ch.to_string(), Style::default().fg(cell.color)))
            .collect();
        all_lines.push(Line::from(spans));
    }

    // Time axis: adaptive label interval
    let label_interval = if visible.len() > 30 {
        6
    } else if visible.len() > 20 {
        5
    } else if visible.len() > 10 {
        3
    } else {
        1
    };

    // Build time labels aligned to candle slots
    let mut time_spans: Vec<Span> = Vec::new();
    let mut col = 0usize;
    for (i, vc) in visible.iter().enumerate() {
        let slot_start = i * real_slot;
        // Pad to slot start
        while col < slot_start && col < cols {
            time_spans.push(Span::styled(" ", Style::default()));
            col += 1;
        }
        if i % label_interval == 0 && slot_start + 4 < cols {
            let ts = vc.timestamp;
            let label = format!("{:02}:{:02}", ts.format("%H"), ts.format("%M"));
            let label = if label.len() > 5 { &label[..5] } else { &label };
            time_spans.push(Span::styled(
                format!("{:<5}", label),
                Style::default().fg(theme::TEXT_FAINT),
            ));
            col += 5;
        } else {
            // Space out to fill the slot
            while col < slot_start + real_slot && col < cols {
                time_spans.push(Span::styled(" ", Style::default()));
                col += 1;
            }
        }
    }
    while col < cols {
        time_spans.push(Span::styled(" ", Style::default()));
        col += 1;
    }
    all_lines.push(Line::from(time_spans));

    // Y-axis labels
    let mut y_labels: Vec<Line> = Vec::with_capacity(chart_rows + sep_rows + vol_rows + time_rows);

    for row in 0..chart_rows {
        let price = row_to_price(row, min_price, price_range, chart_rows, effective_levels);
        let label_step = if chart_rows < 15 { 3 } else { cmp::max(1, chart_rows / 7) };
        let label = if row == 0 || row == chart_rows - 1 || row % label_step == 0 {
            format_price(price)
        } else {
            String::new()
        };
        y_labels.push(Line::from(vec![
            Span::styled(format!("{:>w$}", label, w = y_label_width - 1), Style::default().fg(theme::TEXT_FAINT)),
            Span::styled("┤", Style::default().fg(theme::BORDER_DIM)),
        ]));
    }

    // Separator y-label
    y_labels.push(Line::from(Span::styled(
        format!("{:>w$}┤", "", w = y_label_width - 1),
        Style::default().fg(theme::BORDER_DIM),
    )));

    // Volume y-label rows
    for _ in 0..vol_rows {
        y_labels.push(Line::from(Span::styled(
            format!("{:>w$}┤", "", w = y_label_width - 1),
            Style::default().fg(theme::BORDER_DIM),
        )));
    }
    // Time y-label
    y_labels.push(Line::from(Span::styled(
        format!("{:>w$}┤", "", w = y_label_width - 1),
        Style::default().fg(theme::BORDER_DIM),
    )));

    // Render
    let y_axis = Paragraph::new(y_labels);
    f.render_widget(y_axis, y_axis_area);

    let chart = Paragraph::new(all_lines);
    f.render_widget(chart, chart_area);
}

#[derive(Clone)]
struct VisualCandle {
    open: Decimal,
    high: Decimal,
    low: Decimal,
    close: Decimal,
    volume: Decimal,
    timestamp: chrono::DateTime<chrono::Local>,
}

fn build_visual_candles(candles: &[&Candle], max_slots: usize) -> Vec<VisualCandle> {
    let target = max_slots.max(1);
    if target == 0 { return Vec::new(); }

    if candles.len() <= target {
        return candles.iter().map(|c| VisualCandle {
            open: c.open, high: c.high, low: c.low, close: c.close,
            volume: c.volume, timestamp: c.timestamp,
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
        let low = group.iter().map(|c| c.low).fold(Decimal::MIN, |a, b| a.min(b).max(a));
        let low = if low == Decimal::MIN { group[0].low } else { low };
        let volume: Decimal = group.iter().map(|c| c.volume).sum();
        result.push(VisualCandle {
            open, high, low, close, volume,
            timestamp: group[group.len() - 1].timestamp,
        });
        i = end;
    }
    result
}

fn price_to_eff(price: Decimal, min: Decimal, range: Decimal, effective_levels: usize) -> usize {
    if range == Decimal::ZERO || effective_levels <= 1 {
        return effective_levels.saturating_sub(1);
    }
    let ratio = (price - min) / range;
    let eff_f = ratio.to_f64().unwrap_or(0.0).clamp(0.0, 1.0);
    let eff = (eff_f * (effective_levels - 1) as f64).round() as usize;
    effective_levels.saturating_sub(1).saturating_sub(eff)
}

fn row_to_price(row: usize, min: Decimal, range: Decimal, effective_levels: usize, chart_rows: usize) -> Decimal {
    if chart_rows <= 1 {
        return min;
    }
    // Price at middle of row: considers both halves
    let eff = (chart_rows - 1 - row) * 2 + 1; // middle effective level of this row
    let ratio = Decimal::from_f64_retain(eff as f64 / (effective_levels - 1) as f64)
        .unwrap_or(Decimal::ZERO);
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

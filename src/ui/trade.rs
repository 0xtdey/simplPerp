use crate::app::App;
use crate::ui::theme;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use rust_decimal::prelude::ToPrimitive;

pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(0),
        ])
        .split(area);

    let form = &app.trade_form;
    let is_editing = app.input_mode == crate::app::InputMode::Editing;

    // Side field
    let side_text = format!("1. Side: {:?}", form.side);
    render_field(f, chunks[0], &side_text, form.field == crate::app::TradeField::Side, is_editing);

    // Type field
    let type_text = format!("2. Type: {:?}", form.order_type);
    render_field(f, chunks[1], &type_text, form.field == crate::app::TradeField::Type, is_editing);

    // Price field
    let price_text = if is_editing && form.field == crate::app::TradeField::Price {
        format!("3. Price: {}▌", app.input_buffer)
    } else {
        format!("3. Price: {}", if form.price.is_empty() { "Market" } else { &form.price })
    };
    render_field(f, chunks[2], &price_text, form.field == crate::app::TradeField::Price, is_editing);

    // Size field
    let size_text = if is_editing && form.field == crate::app::TradeField::Size {
        format!("4. Size: {}▌", app.input_buffer)
    } else {
        format!("4. Size: {}", if form.size.is_empty() { "—" } else { &form.size })
    };
    render_field(f, chunks[3], &size_text, form.field == crate::app::TradeField::Size, is_editing);

    // Leverage field
    let lev_text = if is_editing && form.field == crate::app::TradeField::Leverage {
        format!("5. Leverage: {}x▌", app.input_buffer)
    } else {
        format!("5. Leverage: {}x", form.leverage)
    };
    render_field(f, chunks[4], &lev_text, form.field == crate::app::TradeField::Leverage, is_editing);

    // Submit hint
    render_field(f, chunks[5], "  Space/Enter = toggle  |  s = Submit order", false, false);

    // Balance info
    let market = app.engine.current_market();
    let price = market.oracle.price();
    let size_val: Option<f64> = form.size.parse().ok();
    let lev_val: Option<f64> = form.leverage.parse().ok();
    let margin_info = if let (Some(s), Some(l)) = (size_val, lev_val) {
        let notional = s * price.to_f64().unwrap_or(0.0);
        let margin = notional / l;
        format!(
            "Est. Notional: {:.2} | Margin Req: {:.2} | Available: {:.2}",
            notional, margin,
            app.user.available_balance.to_f64().unwrap_or(0.0)
        )
    } else {
        format!(
            "Balance: {:.2} | Available: {:.2}",
            app.user.balance.to_f64().unwrap_or(0.0),
            app.user.available_balance.to_f64().unwrap_or(0.0)
        )
    };

    let info = Paragraph::new(margin_info)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::BORDER)))
        .style(Style::default().fg(theme::TEXT_MUTED));
    f.render_widget(info, chunks[6]);
}

fn render_field(f: &mut Frame, area: Rect, text: &str, selected: bool, editing: bool) {
    let style = if editing && selected {
        Style::default().fg(theme::GREEN).add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
    } else if selected {
        Style::default().fg(theme::AMBER).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme::TEXT_PRIMARY)
    };

    let p = Paragraph::new(text).style(style);
    f.render_widget(p, area);
}

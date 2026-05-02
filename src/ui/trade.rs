use crate::app::{App, InputMode, TradeField};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let market = app.engine.current_market();
    let block = Block::default().borders(Borders::ALL).title(format!("Trade - {}", market.symbol));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let form = &app.trade_form;
    let fields = vec![
        (TradeField::Side, "Side", format!("{:?}", form.side)),
        (TradeField::Type, "Type", format!("{:?}", form.order_type)),
        (TradeField::Price, "Price", form.price.clone()),
        (TradeField::Size, "Size", form.size.clone()),
        (TradeField::Leverage, "Leverage", format!("{}x", form.leverage)),
    ];

    let is_editing = app.input_mode == InputMode::Editing;

    let lines: Vec<Line> = fields
        .into_iter()
        .map(|(field, label, value)| {
            let is_active = form.field == field;
            let is_this_field_editing = is_editing && is_active
                && matches!(field, TradeField::Price | TradeField::Size | TradeField::Leverage);

            let style = if is_this_field_editing {
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
                    .add_modifier(Modifier::UNDERLINED)
            } else if is_active {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let display_value = if is_this_field_editing {
                app.input_buffer.clone()
            } else {
                value
            };

            let suffix = if is_this_field_editing {
                Span::styled(" ▌", Style::default().fg(Color::Green))
            } else {
                Span::raw("")
            };

            Line::from(vec![
                Span::styled(format!("{:10}: ", label), style),
                Span::styled(display_value, style),
                suffix,
            ])
        })
        .collect();

    let paragraph = Paragraph::new(lines)
        .block(Block::default().borders(Borders::NONE))
        .wrap(Wrap { trim: true });
    f.render_widget(paragraph, inner);
}

use crate::app::App;
use crate::ui::theme;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::BORDER))
        .title(" Account ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let positions_count = app.user.positions.len();
    let total_upnl: rust_decimal::Decimal = app
        .user
        .positions
        .values()
        .map(|p| p.unrealized_pnl)
        .sum();

    let upnl_str = if total_upnl >= rust_decimal::Decimal::ZERO {
        format!("+{:.2}", total_upnl)
    } else {
        format!("{:.2}", total_upnl)
    };
    let upnl_color = if total_upnl >= rust_decimal::Decimal::ZERO {
        theme::GREEN
    } else {
        theme::RED
    };

    let lines = vec![
        Line::from(vec![
            Span::styled("   Pubkey: ", Style::default().fg(theme::TEXT_MUTED)),
            Span::styled(&app.user.pubkey, Style::default().fg(theme::TEXT_PRIMARY)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("   Balance:          ", Style::default().fg(theme::TEXT_MUTED)),
            Span::styled(
                format!("{:.2} USDC", app.user.balance),
                Style::default().fg(theme::TEXT_PRIMARY).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("   Available:        ", Style::default().fg(theme::TEXT_MUTED)),
            Span::styled(
                format!("{:.2} USDC", app.user.available_balance),
                Style::default().fg(theme::GREEN),
            ),
        ]),
        Line::from(vec![
            Span::styled("   Margin Used:      ", Style::default().fg(theme::TEXT_MUTED)),
            Span::styled(
                format!("{:.2} USDC", app.user.balance - app.user.available_balance),
                Style::default().fg(theme::RED),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("   Positions:        ", Style::default().fg(theme::TEXT_MUTED)),
            Span::styled(
                format!("{}", positions_count),
                Style::default().fg(theme::TEXT_PRIMARY),
            ),
            Span::styled(
                format!("  Unrealized PnL: {}", upnl_str),
                Style::default().fg(upnl_color).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("   ──", Style::default().fg(theme::BORDER_DIM)),
        ]),
        Line::from(vec![
            Span::styled("   d", Style::default().fg(theme::GREEN).add_modifier(Modifier::BOLD)),
            Span::styled(" → Deposit USDC", Style::default().fg(theme::TEXT_PRIMARY)),
            Span::styled("    ", Style::default()),
            Span::styled("w", Style::default().fg(theme::RED).add_modifier(Modifier::BOLD)),
            Span::styled(" → Withdraw USDC", Style::default().fg(theme::TEXT_PRIMARY)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("   Type amount and press ", Style::default().fg(theme::TEXT_MUTED)),
            Span::styled("Enter", Style::default().fg(theme::AMBER).add_modifier(Modifier::BOLD)),
            Span::styled(", press ", Style::default().fg(theme::TEXT_MUTED)),
            Span::styled("Esc", Style::default().fg(theme::AMBER).add_modifier(Modifier::BOLD)),
            Span::styled(" to cancel", Style::default().fg(theme::TEXT_MUTED)),
        ]),
    ];

    let paragraph = Paragraph::new(lines)
        .block(Block::default().borders(Borders::NONE))
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, inner);
}

use crate::app::App;
use crate::ui::theme;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(8), Constraint::Min(0)])
        .split(area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::BORDER))
        .title(" Account ");
    let inner = block.inner(chunks[0]);
    f.render_widget(block, chunks[0]);

    let text = vec![
        Line::from(vec![
            Span::styled("Pubkey: ", Style::default().fg(theme::TEXT_MUTED)),
            Span::styled(&app.user.pubkey, Style::default().fg(theme::TEXT_PRIMARY)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Balance:          ", Style::default().fg(theme::TEXT_MUTED)),
            Span::styled(
                format!("{:.2}", app.user.balance),
                Style::default().fg(theme::TEXT_PRIMARY).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("Available:        ", Style::default().fg(theme::TEXT_MUTED)),
            Span::styled(
                format!("{:.2}", app.user.available_balance),
                Style::default().fg(theme::GREEN),
            ),
        ]),
        Line::from(vec![
            Span::styled("Margin Used:      ", Style::default().fg(theme::TEXT_MUTED)),
            Span::styled(
                format!("{:.2}", app.user.balance - app.user.available_balance),
                Style::default().fg(theme::RED),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("d → Deposit    w → Withdraw", Style::default().fg(theme::TEXT_FAINT)),
        ]),
    ];

    f.render_widget(Paragraph::new(text), inner);
}

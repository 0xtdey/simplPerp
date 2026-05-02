use crate::app::App;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default().borders(Borders::ALL).title("Account");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let text = vec![
        format!("Pubkey: {}", app.user.pubkey),
        format!("Total Balance:     {:.2} USDC", app.user.balance),
        format!("Available Balance: {:.2} USDC", app.user.available_balance),
        format!("Margin Used:       {:.2} USDC", app.user.balance - app.user.available_balance),
        "".to_string(),
        "Keys: d = Deposit, w = Withdraw".to_string(),
    ];

    let paragraph = Paragraph::new(text.join("\n"))
        .block(Block::default().borders(Borders::NONE))
        .wrap(Wrap { trim: true })
        .style(Style::default().fg(Color::White));
    f.render_widget(paragraph, inner);
}

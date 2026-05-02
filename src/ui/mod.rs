use crate::app::{App, Screen};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Text},
    widgets::{
        Block, Borders, Clear, Paragraph, Tabs, Wrap,
    },
    Frame,
};

pub mod account;
pub mod chart;
pub mod history;
pub mod market;
pub mod positions;
pub mod theme;
pub mod trade;

pub fn render(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0), Constraint::Length(3)].as_ref())
        .split(f.size());

    render_header(f, app, chunks[0]);
    render_main(f, app, chunks[1]);
    render_footer(f, app, chunks[2]);

    if app.input_mode == crate::app::InputMode::Editing && app.current_screen != Screen::Trade {
        render_popup(f, app);
    }
}

fn render_header(f: &mut Frame, app: &App, area: Rect) {
    let titles: Vec<_> = ["Market", "Trade", "Positions", "Account", "History", "Help"]
        .iter()
        .map(|t| Line::from(*t))
        .collect();
    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme::BORDER))
                .title("Terminal Perps"),
        )
        .select(screen_to_index(app.current_screen))
        .style(Style::default().fg(theme::TEXT_PRIMARY))
        .highlight_style(
            Style::default()
                .fg(theme::AMBER)
                .bg(theme::SURFACE)
                .add_modifier(Modifier::BOLD),
        );
    f.render_widget(tabs, area);
}

fn render_main(f: &mut Frame, app: &mut App, area: Rect) {
    match app.current_screen {
        Screen::Market => market::render(f, app, area),
        Screen::Trade => trade::render(f, app, area),
        Screen::Positions => positions::render(f, app, area),
        Screen::Account => account::render(f, app, area),
        Screen::History => history::render(f, app, area),
        Screen::Help => render_help(f, app, area),
    }
}

fn render_footer(f: &mut Frame, app: &App, area: Rect) {
    let market = app.engine.current_market();
    let base = format!(
        "{} | 1-5=Screens, m=Market, Up/Down=Navigate, q=Quit, ?=Help",
        market.symbol
    );
    let text = if let Some(ref msg) = app.message {
        format!("{} | {}", msg, base)
    } else {
        base
    };
    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme::BORDER)),
        )
        .style(Style::default().fg(theme::TEXT_MUTED));
    f.render_widget(paragraph, area);
}

fn render_popup(f: &mut Frame, app: &App) {
    let area = centered_rect(60, 20, f.size());
    let block = Block::default()
        .title("Input")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::AMBER))
        .style(Style::default().bg(theme::BG));
    let input = Paragraph::new(app.input_buffer.as_str())
        .block(block)
        .style(Style::default().fg(theme::AMBER));
    f.render_widget(Clear, area);
    f.render_widget(input, area);
}

fn render_help(f: &mut Frame, _app: &App, area: Rect) {
    let text = Text::from(vec![
        Line::from("Terminal Perps - Help"),
        Line::from(""),
        Line::from("Global:"),
        Line::from("  1-5         - Switch screens (Market, Trade, Positions, Account, History)"),
        Line::from("  m           - Cycle market (BTC/ETH/SOL)"),
        Line::from("  ?           - Help screen"),
        Line::from("  q / Ctrl+C  - Quit"),
        Line::from(""),
        Line::from("Trade Screen:"),
        Line::from("  Up / Down   - Navigate form fields (Side, Type, Price, Size, Leverage)"),
        Line::from("  Space       - Toggle Side (Buy/Sell) or Type (Market/Limit)"),
        Line::from("  Enter       - Edit selected field (Price/Size/Leverage) or Toggle Side/Type"),
        Line::from("  0-9 / .     - Type value while editing"),
        Line::from("  Backspace   - Delete last character while editing"),
        Line::from("  Enter       - Confirm edit"),
        Line::from("  Esc         - Cancel edit"),
        Line::from("  s           - Submit order"),
        Line::from(""),
        Line::from("Market Screen:"),
        Line::from("  Up / Down   - Select order in book"),
        Line::from("  t           - Cycle chart timeframe (1m/5m/15m/1h/4h/1D)"),
        Line::from("  m           - Switch market (BTC/ETH/SOL)"),
        Line::from("  Enter       - Cancel selected order"),
        Line::from(""),
        Line::from("Account Screen:"),
        Line::from("  d           - Deposit"),
        Line::from("  w           - Withdraw"),
    ]);
    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme::BORDER))
                .title("Help"),
        )
        .style(Style::default().fg(theme::TEXT_PRIMARY))
        .wrap(Wrap { trim: true });
    f.render_widget(paragraph, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn screen_to_index(screen: Screen) -> usize {
    match screen {
        Screen::Market => 0,
        Screen::Trade => 1,
        Screen::Positions => 2,
        Screen::Account => 3,
        Screen::History => 4,
        Screen::Help => 5,
    }
}

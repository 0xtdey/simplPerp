use crate::engine::{
    orderbook::{Order, OrderSide, OrderType},
    simulator::MarketSimulator,
    Engine,
};
use crate::persistence;
use crate::user::UserAccount;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use rust_decimal::Decimal;
use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Screen {
    Market,
    Trade,
    Positions,
    Account,
    History,
    Help,
}

pub struct App {
    pub state_path: PathBuf,
    pub current_screen: Screen,
    pub user: UserAccount,
    pub engine: Engine,
    pub selected_row: usize,
    pub input_buffer: String,
    pub input_mode: InputMode,
    pub trade_form: TradeForm,
    pub message: Option<String>,
    pub last_funding_time: chrono::DateTime<chrono::Local>,
    pub ticks: u64,
}

#[derive(Clone, Debug)]
pub struct TradeForm {
    pub side: OrderSide,
    pub order_type: OrderType,
    pub price: String,
    pub size: String,
    pub leverage: String,
    pub field: TradeField,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum TradeField {
    Side,
    Type,
    Price,
    Size,
    Leverage,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum InputMode {
    Normal,
    Editing,
}

impl Default for TradeForm {
    fn default() -> Self {
        Self {
            side: OrderSide::Buy,
            order_type: OrderType::Market,
            price: String::new(),
            size: String::new(),
            leverage: "5".to_string(),
            field: TradeField::Side,
        }
    }
}

impl App {
    pub fn new(state_path: PathBuf) -> Self {
        let mut engine = Engine::new();
        bootstrap_market_data(&mut engine);

        Self {
            state_path,
            current_screen: Screen::Market,
            user: UserAccount::new(),
            engine,
            selected_row: 0,
            input_buffer: String::new(),
            input_mode: InputMode::Normal,
            trade_form: TradeForm::default(),
            message: Some("Welcome to Terminal Perps! Press '?' for help.".to_string()),
            last_funding_time: chrono::Local::now(),
            ticks: 0,
        }
    }

    pub fn on_tick(&mut self) {
        self.ticks += 1;

        // Live simulation tick for current market
        let market = self.engine.current_market_mut();
        let last_price = market.oracle.price();
        let mut rng = rand::thread_rng();
        let (new_price, volume) = market.simulator.tick_live(last_price, &mut rng);
        market.oracle.set_price(new_price);
        market.chart.tick(new_price, volume);

        // Update 24h stats
        market.stats_24h.update(new_price, volume);
        market.stats_24h.maybe_reset();

        // Simulate orderbook activity (real-time depth)
        let mark = market.oracle.price();
        let book = &mut market.orderbook;
        market.simulator.tick_orderbook(book, &mut rng, mark);

        // Apply funding every ~60 seconds (simulated)
        let now = chrono::Local::now();
        if (now - self.last_funding_time).num_seconds() >= 60 {
            let funding = self.engine.funding.calculate_funding(
                self.engine.current_market().oracle.price(),
                self.engine.current_market().oracle.index_price(),
            );
            self.engine
                .funding
                .apply_funding(&mut self.user, funding, self.engine.current_market().oracle.price());
            self.last_funding_time = now;
            self.message = Some(format!("Funding applied: {:.4}%", funding * Decimal::from(100)));
        }

        // Run liquidator
        let price = self.engine.current_market().oracle.price();
        let maint = self.engine.liquidator.maintenance_margin;
        let orderbook = &mut self.engine.current_market_mut().orderbook;
        crate::engine::liquidator::check_liquidations(maint, &mut self.user, price, orderbook);

        // Save periodically
        if self.ticks % 600 == 0 {
            let _ = persistence::save(self);
        }
    }

    pub async fn handle_input(&mut self, key: KeyEvent) -> anyhow::Result<bool> {
        if key.code == KeyCode::Char('c') && key.modifiers == KeyModifiers::CONTROL {
            return Ok(true);
        }

        if key.code == KeyCode::Char('q') && self.input_mode == InputMode::Normal {
            return Ok(true);
        }

        match self.input_mode {
            InputMode::Normal => self.handle_normal_input(key).await,
            InputMode::Editing => self.handle_editing_input(key),
        }

        Ok(false)
    }

    async fn handle_normal_input(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up => {
                if self.current_screen == Screen::Trade {
                    self.cycle_trade_field(-1);
                } else {
                    self.selected_row = self.selected_row.saturating_sub(1);
                }
            }
            KeyCode::Down => {
                if self.current_screen == Screen::Trade {
                    self.cycle_trade_field(1);
                } else {
                    self.selected_row = self.selected_row.saturating_add(1);
                }
            }

            KeyCode::Char('?') => self.current_screen = Screen::Help,
            KeyCode::Char(' ') => {
                if self.current_screen == Screen::Trade {
                    self.toggle_trade_field();
                }
            }
            KeyCode::Enter => match self.current_screen {
                Screen::Trade => {
                    match self.trade_form.field {
                        TradeField::Price | TradeField::Size | TradeField::Leverage => {
                            self.input_mode = InputMode::Editing;
                            self.input_buffer = match self.trade_form.field {
                                TradeField::Price => self.trade_form.price.clone(),
                                TradeField::Size => self.trade_form.size.clone(),
                                TradeField::Leverage => self.trade_form.leverage.clone(),
                                _ => String::new(),
                            };
                        }
                        _ => self.toggle_trade_field(),
                    }
                }
                Screen::Market => self.cancel_selected_order(),
                _ => {}
            },
            KeyCode::Char('s') => {
                if self.current_screen == Screen::Trade {
                    self.submit_trade();
                }
            }
            KeyCode::Char('t') => {
                if self.current_screen == Screen::Market {
                    let next = self.engine.current_market().chart.timeframe.next();
                    let market = self.engine.current_market_mut();
                    market.chart.set_timeframe(next);
                    self.message = Some(format!("Timeframe: {}", next.label()));
                }
            }
            KeyCode::Char('m') => {
                self.engine.next_market();
                let sym = self.engine.current_market.clone();
                self.message = Some(format!("Market: {}", sym));
            }

            KeyCode::Char('d') => {
                if self.current_screen == Screen::Account {
                    self.input_mode = InputMode::Editing;
                    self.input_buffer.clear();
                    self.message = Some("Enter amount to deposit:".to_string());
                }
            }
            KeyCode::Char('w') => {
                if self.current_screen == Screen::Account {
                    self.input_mode = InputMode::Editing;
                    self.input_buffer.clear();
                    self.message = Some("Enter amount to withdraw:".to_string());
                }
            }

            KeyCode::Char(c) => {
                if self.current_screen == Screen::Trade {
                    match self.trade_form.field {
                        TradeField::Price | TradeField::Size | TradeField::Leverage => {
                            if c.is_ascii_digit() || c == '.' {
                                self.input_mode = InputMode::Editing;
                                self.input_buffer.clear();
                                self.input_buffer.push(c);
                                return;
                            }
                        }
                        _ => {}
                    }
                }

                match c {
                    '1' => self.current_screen = Screen::Market,
                    '2' => self.current_screen = Screen::Trade,
                    '3' => self.current_screen = Screen::Positions,
                    '4' => self.current_screen = Screen::Account,
                    '5' => self.current_screen = Screen::History,
                    _ => {}
                }
            }

            _ => {}
        }
    }

    fn handle_editing_input(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter => {
                let buf = self.input_buffer.clone();
                if self.current_screen == Screen::Account {
                    if let Ok(amount) = buf.parse::<Decimal>() {
                        if self.message.as_ref().map_or(false, |m| m.contains("deposit")) {
                            self.user.deposit(amount);
                            self.message = Some(format!("Deposited {}", amount));
                        } else if self.message.as_ref().map_or(false, |m| m.contains("withdraw")) {
                            if self.user.withdraw(amount) {
                                self.message = Some(format!("Withdrew {}", amount));
                            } else {
                                self.message = Some("Insufficient balance".to_string());
                            }
                        }
                    }
                } else if self.current_screen == Screen::Trade {
                    match self.trade_form.field {
                        TradeField::Price => self.trade_form.price = buf,
                        TradeField::Size => self.trade_form.size = buf,
                        TradeField::Leverage => self.trade_form.leverage = buf,
                        _ => {}
                    }
                }
                self.input_mode = InputMode::Normal;
                self.input_buffer.clear();
            }
            KeyCode::Char(c) => self.input_buffer.push(c),
            KeyCode::Backspace => {
                self.input_buffer.pop();
            }
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
                self.input_buffer.clear();
            }
            _ => {}
        }
    }

    fn cycle_trade_field(&mut self, delta: i8) {
        let fields = [
            TradeField::Side,
            TradeField::Type,
            TradeField::Price,
            TradeField::Size,
            TradeField::Leverage,
        ];
        let current = fields.iter().position(|&f| f == self.trade_form.field).unwrap_or(0);
        let next = (current as i8 + delta).rem_euclid(fields.len() as i8) as usize;
        self.trade_form.field = fields[next];
    }

    fn toggle_trade_field(&mut self) {
        match self.trade_form.field {
            TradeField::Side => {
                self.trade_form.side = match self.trade_form.side {
                    OrderSide::Buy => OrderSide::Sell,
                    OrderSide::Sell => OrderSide::Buy,
                }
            }
            TradeField::Type => {
                self.trade_form.order_type = match self.trade_form.order_type {
                    OrderType::Market => OrderType::Limit,
                    OrderType::Limit => OrderType::Market,
                }
            }
            _ => {}
        }
    }

    fn submit_trade(&mut self) {
        let market = self.engine.current_market();
        let price = if self.trade_form.order_type == OrderType::Market {
            match self.trade_form.side {
                OrderSide::Buy => market.orderbook.best_ask().unwrap_or(market.oracle.price()),
                OrderSide::Sell => market.orderbook.best_bid().unwrap_or(market.oracle.price()),
            }
        } else {
            match self.trade_form.price.parse::<Decimal>() {
                Ok(p) => p,
                Err(_) => {
                    self.message = Some("Invalid price".to_string());
                    return;
                }
            }
        };

        let size = match self.trade_form.size.parse::<Decimal>() {
            Ok(s) if s > Decimal::ZERO => s,
            _ => {
                self.message = Some("Invalid size".to_string());
                return;
            }
        };

        let leverage = match self.trade_form.leverage.parse::<u64>() {
            Ok(l) if l >= 1 && l <= 20 => l,
            _ => {
                self.message = Some("Leverage must be 1-20x".to_string());
                return;
            }
        };

        let notional = size * price;
        let margin_required = notional / Decimal::from(leverage);

        if self.user.available_balance < margin_required {
            self.message = Some(format!(
                "Insufficient margin. Required: {}, Available: {}",
                margin_required, self.user.available_balance
            ));
            return;
        }

        let order = Order::new(
            self.user.pubkey.clone(),
            self.trade_form.side,
            self.trade_form.order_type,
            price,
            size,
            leverage as u32,
        );

        let fills = self.engine.submit_order(order);

        let mut total_volume = Decimal::ZERO;
        for fill in &fills {
            self.user.apply_fill(fill, self.engine.current_market().oracle.price());
            self.engine.add_recent_trade(fill.price, fill.size, fill.side);
            total_volume += fill.size;
        }
        if total_volume > Decimal::ZERO {
            self.engine.tick_chart(self.engine.current_market().oracle.price(), total_volume);
        }

        self.message = Some(format!(
            "Order submitted: {:?} {} @ {} with {}x leverage on {}",
            self.trade_form.side, size, price, leverage, self.engine.current_market
        ));
    }

    fn cancel_selected_order(&mut self) {
        let market = self.engine.current_market();
        let orders: Vec<_> = market.orderbook.user_orders(&self.user.pubkey);
        if let Some(order_id) = orders.get(self.selected_row).map(|o| o.id) {
            if self.engine.cancel_order(order_id) {
                self.message = Some("Order cancelled".to_string());
            }
        }
    }
}

fn bootstrap_market_data(engine: &mut Engine) {
    let data_dir = PathBuf::from(".terminal-perps/markets");
    let _ = std::fs::create_dir_all(&data_dir);

    for (symbol, market) in engine.markets.iter_mut() {
        let path = data_dir.join(format!("{}_30d.json", symbol.to_lowercase().replace("-", "_")));

        let candles = if path.exists() {
            MarketSimulator::load_history(&path).unwrap_or_default()
        } else {
            let generated = market.simulator.generate_history();
            let _ = MarketSimulator::save_history(&generated, &path);
            generated
        };

        market.load_history(&candles);
    }
}

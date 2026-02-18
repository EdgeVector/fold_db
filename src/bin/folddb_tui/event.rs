use crossterm::event::{self, KeyEvent, KeyEventKind};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

pub enum Event {
    Key(KeyEvent),
    Tick,
}

pub struct EventHandler {
    rx: mpsc::Receiver<Event>,
    _tick_rate: Duration,
}

impl EventHandler {
    pub fn new(tick_rate_ms: u64) -> Self {
        let tick_rate = Duration::from_millis(tick_rate_ms);
        let (tx, rx) = mpsc::channel();

        let event_tx = tx.clone();
        thread::spawn(move || loop {
            if event::poll(tick_rate).unwrap_or(false) {
                if let Ok(crossterm::event::Event::Key(key)) = event::read() {
                    // Only handle key press events (not release/repeat)
                    if key.kind == KeyEventKind::Press
                        && event_tx.send(Event::Key(key)).is_err()
                    {
                        return;
                    }
                }
            }
            if tx.send(Event::Tick).is_err() {
                return;
            }
        });

        Self {
            rx,
            _tick_rate: tick_rate,
        }
    }

    pub fn next(&self) -> Result<Event, mpsc::RecvError> {
        self.rx.recv()
    }
}

use anyhow::Result;
use crossterm::event::{self, Event, KeyEvent, KeyEventKind};
use std::time::Duration;

pub enum AppEvent {
    Key(KeyEvent),
    Resize(u16, u16),
    Tick,
}

pub struct EventReader;

impl EventReader {
    pub fn new() -> Self {
        Self
    }

    pub fn next(&self) -> Result<AppEvent> {
        if event::poll(Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    return Ok(AppEvent::Key(key));
                }
                Event::Resize(w, h) => {
                    return Ok(AppEvent::Resize(w, h));
                }
                _ => {}
            }
        }
        Ok(AppEvent::Tick)
    }
}

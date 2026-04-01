use anyhow::Result;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;

use myro_tui::app::App;
use myro_tui::event::{AppEvent, EventReader};
use myro_tui::ui;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(|s| s.as_str()) {
        Some("update") => myro_tui::updater::run_update(),
        Some("version") | Some("--version") | Some("-V") => {
            println!("myro {}", myro_tui::updater::CURRENT_VERSION);
            Ok(())
        }
        _ => run_tui(),
    }
}

fn run_tui() -> Result<()> {
    // Init terminal
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run(&mut terminal);

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run(terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>) -> Result<()> {
    let mut app = if std::env::var("MYRO_EPHEMERAL").is_ok() {
        App::new_ephemeral()?
    } else {
        App::new()?
    };
    let events = EventReader::new();

    loop {
        terminal.draw(|frame| ui::render(frame, &mut app))?;

        match events.next()? {
            AppEvent::Key(key) => app.handle_key(key),
            AppEvent::Resize(w, h) => {
                app.terminal_width = w;
                app.terminal_height = h;
            }
            AppEvent::Tick => app.tick(),
        }

        if app.should_quit {
            // Flush analytics on exit
            if let Some(ref events) = app.events {
                events.track("session_end", serde_json::json!({}));
                let _ = events.flush();
            }
            break;
        }
    }

    Ok(())
}

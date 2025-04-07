use std::io;
use std::os::unix::process::CommandExt;
use std::process::Command;

use crate::app::App;

mod app;
mod ui;

use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::prelude::CrosstermBackend;
use ratatui::Terminal;

use color_eyre::Result;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    color_eyre::install()?;

    enable_raw_mode()?;
    let mut stderr = io::stderr();
    execute!(stderr, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stderr);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    app.read_ssh_conf()?;
    let selected_id = app.run(&mut terminal);

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    if let Ok(id) = selected_id {
        execute_ssh(id);
    }

    Ok(())
}

fn execute_ssh(id: String) {
    Command::new("ssh").arg("-t").arg(id).exec();
}

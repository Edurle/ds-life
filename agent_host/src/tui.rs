use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use std::io;

pub fn run_tui() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut input_buffer = String::new();
    let mut output_text =
        String::from("Agent TUI ready. Type your message and press Enter.\n");

    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints(
                    [
                        Constraint::Min(3),
                        Constraint::Length(3),
                    ]
                    .as_ref(),
                )
                .split(f.size());

            let output = Paragraph::new(output_text.clone())
                .block(Block::default().title("Output").borders(Borders::ALL));
            f.render_widget(output, chunks[0]);

            let input = Paragraph::new(input_buffer.clone()).block(
                Block::default()
                    .title("Input (Enter to send, Esc to quit)")
                    .borders(Borders::ALL),
            );
            f.render_widget(input, chunks[1]);
        })?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Esc => break,
                    KeyCode::Enter => {
                        if !input_buffer.is_empty() {
                            output_text.push_str(&format!("\n> {}\n", input_buffer));
                            output_text.push_str("(Agent processing...)\n");
                            input_buffer.clear();
                        }
                    }
                    KeyCode::Char(c) => {
                        input_buffer.push(c);
                    }
                    KeyCode::Backspace => {
                        input_buffer.pop();
                    }
                    _ => {}
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}

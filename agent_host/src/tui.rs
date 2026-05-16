use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use std::io;

struct TuiState {
    output_text: String,
    input_buffer: String,
    scroll_offset: usize,
    processing: bool,
    done: bool,
    demo: bool,
}

impl TuiState {
    fn new(demo: bool) -> Self {
        Self {
            output_text: String::from("Agent TUI ready. Type /help for commands.\n"),
            input_buffer: String::new(),
            scroll_offset: 0,
            processing: false,
            done: false,
            demo,
        }
    }

    fn push_output(&mut self, line: &str) {
        self.output_text.push_str(line);
        self.output_text.push('\n');
    }

    fn push_cmd_output(&mut self, text: &str) {
        self.push_output(&format!("#  {}", text));
    }
}

/// 运行 TUI 交互模式。`demo = true` 时不调用 Agent API，仅模拟。
pub async fn run_tui(demo: bool) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut state = TuiState::new(demo);

    if demo {
        state.push_output("[Demo mode] No API calls will be made.");
        state.push_output("Try: type a message (echo), or use /help to see commands.");
    }

    loop {
        // ── 绘制 ──
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([Constraint::Percentage(85), Constraint::Length(3)])
                .split(f.size());

            let output_title = if state.processing {
                "Output (Agent is thinking...)"
            } else {
                "Output"
            };

            let panel_height = chunks[0].height.saturating_sub(2) as usize;
            let total_lines = state.output_text.lines().count();
            let max_scroll = total_lines.saturating_sub(panel_height);
            let scroll = state.scroll_offset.min(max_scroll);

            let output = Paragraph::new(state.output_text.clone())
                .scroll((scroll as u16, 0))
                .style(Style::default().fg(if state.demo {
                    Color::DarkGray
                } else {
                    Color::Reset
                }))
                .block(Block::default().title(output_title).borders(Borders::ALL));
            f.render_widget(output, chunks[0]);

            let input_title = if state.processing {
                "Agent is thinking..."
            } else {
                "Input (Enter to send, Esc to quit)"
            };
            let input = Paragraph::new(state.input_buffer.clone())
                .block(Block::default().title(input_title).borders(Borders::ALL));
            f.render_widget(input, chunks[1]);
        })?;

        // ── 等待 Agent ──
        if state.processing {
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
            continue;
        }

        // ── 键盘事件 ──
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Esc => break,
                    KeyCode::Enter => {
                        if state.input_buffer.is_empty() {
                            continue;
                        }
                        let task = std::mem::take(&mut state.input_buffer);

                        if task.starts_with('/') {
                            state.processing = true;
                            let _ = terminal.draw(|_| {});
                            process_command(&task, &mut state).await;
                            state.processing = false;
                        } else {
                            state.push_output(&format!("> {}", task));
                            state.processing = true;
                            let _ = terminal.draw(|_| {});

                            if state.demo {
                                tokio::time::sleep(
                                    tokio::time::Duration::from_millis(800),
                                )
                                .await;
                                state
                                    .push_output(&format!("(Demo Echo) You said: {}", task));
                            } else {
                                match crate::run_agent_once(&task).await {
                                    Ok(resp) => state.push_output(&resp),
                                    Err(e) => {
                                        state.push_output(&format!("(Error: {})", e));
                                    }
                                }
                            }
                            state.processing = false;
                        }
                        state.scroll_offset = usize::MAX;
                    }
                    KeyCode::PageUp => {
                        let h = terminal.size()?.height.saturating_sub(6) as usize;
                        state.scroll_offset = state.scroll_offset.saturating_add(h);
                    }
                    KeyCode::PageDown => {
                        let h = terminal.size()?.height.saturating_sub(6) as usize;
                        state.scroll_offset = state.scroll_offset.saturating_sub(h);
                    }
                    KeyCode::Home => state.scroll_offset = usize::MAX,
                    KeyCode::End => state.scroll_offset = 0,
                    KeyCode::Char(c) => state.input_buffer.push(c),
                    KeyCode::Backspace => {
                        state.input_buffer.pop();
                    }
                    _ => {}
                }
            }
        }

        if state.done {
            break;
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}

async fn process_command(cmd: &str, state: &mut TuiState) {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    let command = parts[0];

    match command {
        "/clear" | "/c" => state.output_text.clear(),
        "/help" | "/?" | "/h" => {
            state.push_output("");
            state.push_cmd_output("Available commands:");
            state.push_cmd_output("/clear, /c       - Clear output panel");
            state.push_cmd_output("/echo <text>     - Echo text back");
            state.push_cmd_output("/help, /?, /h    - Show this help");
            state.push_cmd_output("/quit, /q, /exit - Exit program");
            state.push_output("");
            if state.demo {
                state.push_cmd_output("(Demo mode: no API calls)");
            }
        }
        "/echo" | "/e" => {
            let text = parts[1..].join(" ");
            if text.is_empty() {
                state.push_cmd_output("Usage: /echo <text>");
            } else {
                state.push_output(&format!("echo: {}", text));
            }
        }
        "/quit" | "/q" | "/exit" => state.done = true,
        _ => {
            state.push_cmd_output(&format!("Unknown command: {}", command));
            state.push_cmd_output("Type /help to see available commands.");
        }
    }
}

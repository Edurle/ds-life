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
use std::sync::{Arc, Mutex};

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
        let mut s = Self {
            output_text: String::new(),
            input_buffer: String::new(),
            scroll_offset: usize::MAX,
            processing: false,
            done: false,
            demo,
        };
        s.push_output("Agent TUI ready. Type /help for commands.");
        if demo {
            s.push_output("[Demo mode] No API calls will be made.");
            s.push_output("Try: type a message, or use /help to see commands.");
        }
        s
    }

    fn push_output(&mut self, line: &str) {
        self.output_text.push_str(line);
        self.output_text.push('\n');
    }

    fn push_cmd_output(&mut self, text: &str) {
        self.push_output(&format!("#  {}", text));
    }
}

fn clamp_scroll(scroll: usize, total_lines: usize, panel_height: usize) -> usize {
    if total_lines <= panel_height {
        return 0;
    }
    let max_scroll = total_lines - panel_height;
    scroll.min(max_scroll)
}

pub async fn run_tui(demo: bool) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut state = TuiState::new(demo);
    let token_buffer: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));

    loop {
        let term_size = terminal.size()?;
        let panel_height = term_size.height.saturating_sub(6) as usize;

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

            let total_lines = state.output_text.lines().count();
            let scroll = clamp_scroll(state.scroll_offset, total_lines, panel_height);
            state.scroll_offset = scroll;

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

        // 处理流式 token（processing 期间持续读取）
        if state.processing && !state.demo {
            stream_token_buffer(&mut state, &token_buffer);
        }

        // processing 时继续循环（保持 draw 更新界面）
        if state.processing {
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
            if state.done {
                break;
            }
            continue;
        }

        // 键盘事件
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
                            process_command(&task, &mut state).await;
                            state.processing = false;
                        } else {
                            state.push_output(&format!("> {}", task));
                            state.processing = true;

                            if state.demo {
                                tokio::time::sleep(
                                    tokio::time::Duration::from_millis(800),
                                )
                                .await;
                                state
                                    .push_output(&format!("(Demo Echo) You said: {}", task));
                            } else {
                                // 设置流式钩子：token 实时写入缓冲区
                                let tb = token_buffer.clone();
                                crate::tools::llm::set_streaming_hook(Box::new(move |token| {
                                    tb.lock().unwrap().push_str(token);
                                }));
                                match crate::run_agent_once(&task).await {
                                    Ok(resp) => state.push_output(&resp),
                                    Err(e) => {
                                        state.push_output(&format!("(Error: {})", e));
                                    }
                                }
                                crate::tools::llm::clear_streaming_hook();
                            }
                            state.processing = false;
                        }
                        state.scroll_offset = usize::MAX;
                    }
                    // scroll 值越大越往底部（更新内容）
                    // PageUp：看更早内容 → scroll 减小
                    KeyCode::PageUp => {
                        state.scroll_offset =
                            state.scroll_offset.saturating_sub(panel_height);
                    }
                    // PageDown：看更新内容 → scroll 增大
                    KeyCode::PageDown => {
                        state.scroll_offset =
                            state.scroll_offset.saturating_add(panel_height);
                    }
                    KeyCode::Home => state.scroll_offset = 0,
                    KeyCode::End => state.scroll_offset = usize::MAX,
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

/// 从流式缓冲区读取 token 并追加到输出面板。
fn stream_token_buffer(state: &mut TuiState, buffer: &Arc<Mutex<String>>) {
    let s = {
        let mut buf = buffer.lock().unwrap();
        if buf.is_empty() {
            return;
        }
        std::mem::take(&mut *buf)
    };
    state.output_text.push_str(&s);
}

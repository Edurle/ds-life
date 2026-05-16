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

/// 运行 TUI 交互模式。每次用户在输入框按 Enter，
/// 将消息发送给 Agent 处理，结果追加到输出面板。
pub async fn run_tui() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut input_buffer = String::new();
    let mut output_text =
        String::from("Agent TUI ready. Type your message and press Enter.\n");
    let mut processing = false;

    // cargo run tui -- "task" 由 main.rs 处理（单次任务模式）
    //      cargo run tui                      （交互模式）

    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([Constraint::Min(3), Constraint::Length(3)].as_ref())
                .split(f.size());

            let output = Paragraph::new(output_text.clone())
                .block(Block::default().title("Output").borders(Borders::ALL));
            f.render_widget(output, chunks[0]);

            let input = Paragraph::new(input_buffer.clone()).block(
                Block::default()
                    .title(if processing {
                        "Agent is thinking..."
                    } else {
                        "Input (Enter to send, Esc to quit)"
                    })
                    .borders(Borders::ALL),
            );
            f.render_widget(input, chunks[1]);
        })?;

        if processing {
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
            continue;
        }

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Esc => break,
                    KeyCode::Enter => {
                        if !input_buffer.is_empty() {
                            let task = std::mem::take(&mut input_buffer);
                            output_text.push_str(&format!("\n> {}\n", task));
                            output_text.push_str("(Agent processing...)\n");
                            // 立刻刷新，显示 "Agent processing..."
                            // processing 标记在此处用于提示栏显示，不会在此分支外被读取两次
                            #[allow(unused_assignments)]
                            {
                                processing = true;
                            }
                            let _ = terminal.draw(|f| {
                                let chunks = Layout::default()
                                    .direction(Direction::Vertical)
                                    .margin(1)
                                    .constraints(
                                        [Constraint::Min(3), Constraint::Length(3)].as_ref(),
                                    )
                                    .split(f.size());
                                let output = Paragraph::new(output_text.clone())
                                    .block(
                                        Block::default()
                                            .title("Output")
                                            .borders(Borders::ALL),
                                    );
                                f.render_widget(output, chunks[0]);
                            });

                            // 调用 Agent
                            match crate::run_agent_once(&task).await {
                                Ok(resp) => {
                                    output_text.push_str(&format!("{}\n", resp));
                                }
                                Err(e) => {
                                    output_text.push_str(&format!("(Error: {})\n", e));
                                }
                            }
                            processing = false;
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

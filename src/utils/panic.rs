use crate::terminal::buffer::InputMode;
use crate::terminal::renderer::LineOptions;
use crate::terminal::{buffer, Terminal};

pub async fn trigger(terminal: &Terminal) {
    buffer::clear_buffer();
    buffer::set_input_mode(InputMode::Disabled);

    let panic_lines = vec![
        ("CRITICAL SYSTEM ERROR", "error"),
        ("", ""),
        ("Deleting root filesystem...", "warning"),
        ("rm: removing /usr... ████████████░░░░ 75%", "warning"),
        ("rm: removing /var... ██████████████░░ 87%", "warning"),
        ("rm: removing /etc... ████████████████ 100%", "warning"),
        ("", ""),
        ("Filesystem table corrupted.", "error"),
        ("Kernel panic: Attempted to kill init!", "error"),
        ("System halt issued.", "error"),
        ("", ""),
        ("Emergency shutdown in 3...", "warning"),
        ("Emergency shutdown in 2...", "warning"),
        ("Emergency shutdown in 1...", "warning"),
        ("", ""),
        ("Shutdown failed. Recovery was possible.", "success"),
    ];

    for (line, color) in panic_lines {
        let options = if line.starts_with("rm: removing") {
            None
        } else {
            Some(LineOptions::new().with_typing(20).with_color(color))
        };

        terminal.add_line(line, options).await;
        terminal.render();
        terminal.sleep(700).await;
    }

    terminal.sleep(1000).await;
    buffer::clear_buffer();
    terminal.render();
}

pub fn should_panic(input: &str) -> bool {
    let parts: Vec<&str> = input.split_whitespace().collect();

    if parts.len() >= 4 && parts[0] == "sudo" && parts[1] == "rm" {
        let has_rf_flag = parts.iter().any(|&part| {
            part == "-rf" || part == "-fr" || part.contains("rf") || part.contains("fr")
        });

        if has_rf_flag {
            for &part in &parts[3..] {
                match part {
                    "/" => return true,
                    "./" => {
                        use crate::commands::filesystem::CURRENT_PATH;
                        let current_path = CURRENT_PATH.lock().unwrap();
                        if current_path.is_empty() {
                            return true;
                        }
                    }
                    path if path.starts_with("./") => {
                        use crate::commands::filesystem::CURRENT_PATH;
                        let current_path = CURRENT_PATH.lock().unwrap();
                        if current_path.is_empty() {
                            return true;
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    false
}

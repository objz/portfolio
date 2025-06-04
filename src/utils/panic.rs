use crate::terminal::buffer::InputMode;
use crate::terminal::{buffer, Terminal};
use crate::utils::panic::buffer::LineType;

pub async fn trigger(terminal: &Terminal) {
    buffer::clear_buffer();
    buffer::set_input_mode(InputMode::Disabled);

    let panic_lines = vec![
        ("⚠️  CRITICAL SYSTEM ERROR ⚠️", Some("error")),
        ("", None),
        ("Deleting root filesystem...", Some("warning")),
        ("rm: removing /usr... ████████████░░░░ 75%", Some("warning")),
        ("rm: removing /var... ██████████████░░ 87%", Some("warning")),
        (
            "rm: removing /etc... ████████████████ 100%",
            Some("warning"),
        ),
        ("", None),
        ("SYSTEM DESTROYED", Some("error")),
        ("", None),
        (
            "Just kidding! This is a just simulated, not your actual system.",
            Some("success"),
        ),
        ("Nice try though!", Some("success")),
        ("", None),
        (
            "(Don't actually run 'sudo rm -rf /' on real systems!)",
            Some("warning"),
        ),
    ];

    for (line, color) in panic_lines {
        buffer::add_line(
            line.to_string(),
            LineType::System,
            color.map(|s| s.to_string()),
        );
        terminal.render();
        terminal.sleep(1400).await;
    }

    terminal.sleep(2000).await;

    buffer::clear_buffer();
    buffer::add_line(
        "System restored! Terminal is back online.".to_string(),
        LineType::System,
        Some("success".to_string()),
    );
    buffer::add_line("".to_string(), LineType::Normal, None);
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

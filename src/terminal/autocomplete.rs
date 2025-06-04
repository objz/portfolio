pub struct AutoComplete {
    commands: Vec<String>,
}

impl AutoComplete {
    pub fn new() -> Self {
        let commands = vec![
            "help", "clear", "history", "echo", "date", "uptime", "neofetch", "uname", "ls", "ll",
            "cd", "cat", "pwd", "tree", "mkdir", "touch", "rm", "ln", "sudo", "cowsay", "sl",
            "lolcat", "calc",
        ]
        .into_iter()
        .map(|s| s.to_string())
        .collect();

        Self { commands }
    }

    pub fn complete(&mut self, input: &str, current_path: &[String]) -> CompletionResult {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return CompletionResult::None;
        }

        let parts: Vec<&str> = trimmed.split_whitespace().collect();

        if parts.len() == 1 {
            self.complete_command(&parts[0])
        } else {
            let command = parts[0];
            let partial_path = parts.last().map_or("", |v| v);

            match command {
                "cd" | "ls" | "cat" | "tree" | "rm" | "mkdir" | "touch" => {
                    self.complete_path(partial_path, current_path, command == "cd")
                }
                _ => CompletionResult::None,
            }
        }
    }

    fn complete_command(&self, partial: &str) -> CompletionResult {
        let matches: Vec<String> = self
            .commands
            .iter()
            .filter(|cmd| cmd.starts_with(partial))
            .cloned()
            .collect();

        match matches.len() {
            0 => CompletionResult::None,
            1 => CompletionResult::Single(matches[0].clone()),
            _ => CompletionResult::Multiple(matches),
        }
    }

    fn complete_path(
        &mut self,
        partial: &str,
        current_path: &[String],
        dirs_only: bool,
    ) -> CompletionResult {
        use crate::commands::filesystem::{get_filesystem_entries, normalize_path};

        let (dir_path, filename_prefix) = if partial.contains('/') {
            let last_slash = partial.rfind('/').unwrap();
            (&partial[..last_slash + 1], &partial[last_slash + 1..])
        } else {
            ("", partial)
        };

        let search_path = if dir_path.is_empty() {
            current_path.to_vec()
        } else {
            normalize_path(dir_path, current_path)
        };

        let entries = get_filesystem_entries(&search_path, dirs_only);

        let matches: Vec<String> = entries
            .into_iter()
            .filter(|entry| entry.starts_with(filename_prefix))
            .map(|entry| {
                if dir_path.is_empty() {
                    entry
                } else {
                    format!("{}{}", dir_path, entry)
                }
            })
            .collect();

        match matches.len() {
            0 => CompletionResult::None,
            1 => CompletionResult::Single(matches[0].clone()),
            _ => CompletionResult::Multiple(matches),
        }
    }
}

#[derive(Debug)]
pub enum CompletionResult {
    None,
    Single(String),
    Multiple(Vec<String>),
}

pub fn find_common_prefix(strings: &[String]) -> Option<String> {
    if strings.is_empty() {
        return None;
    }

    let first = &strings[0];
    let mut prefix = String::new();

    for (i, ch) in first.chars().enumerate() {
        if strings.iter().all(|s| s.chars().nth(i) == Some(ch)) {
            prefix.push(ch);
        } else {
            break;
        }
    }

    if prefix.is_empty() {
        None
    } else {
        Some(prefix)
    }
}

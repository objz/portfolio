use crate::commands::system;

use super::{commands, misc};

#[derive(Clone)]
pub struct CommandHandler {
    history: Vec<String>,
}

impl CommandHandler {
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
        }
    }

    pub fn get_current_directory(&self) -> String {
        commands::pwd(&[])
    }

    pub fn handle(&mut self, input: &str) -> (String, bool) {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return (String::new(), false);
        }

        self.history.push(trimmed.to_string());
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        let cmd = parts[0];
        let args = &parts[1..];

        let directory_changed = cmd == "cd";

        let output = match cmd {
            "clear" => system::clear(args),
            "history" => self.show_history(args),
            "echo" => system::echo(args),
            "date" => system::date(args),
            "uptime" => system::uptime(args),
            "neofetch" => system::neofetch(args),

            "ls" => commands::ls(args),
            "cd" => commands::cd(args),
            "cat" => commands::cat(args),
            "pwd" => commands::pwd(args),
            "tree" => commands::tree(args),
            "mkdir" => commands::mkdir(args),
            "touch" => commands::touch(args),
            "rm" => commands::rm(args),
            "uname" => commands::uname(args),
            "ln" => commands::ln(args),
            "ll" => commands::ls(&["-la"]),

            "help" => misc::help(args),
            "sudo" => misc::sudo(args),
            "cowsay" => misc::cowsay(args),
            "sl" => misc::sl(args),
            "lolcat" => misc::lolcat(args),
            "calc" => misc::calc(args),

            _ => format!("zsh: command not found: {}", cmd),
        };

        (output, directory_changed)
    }

    fn show_history(&self, _args: &[&str]) -> String {
        if self.history.is_empty() {
            "No commands in history yet.".to_string()
        } else {
            self.history
                .iter()
                .enumerate()
                .map(|(i, cmd)| format!("  {}  {}", i + 1, cmd))
                .collect::<Vec<_>>()
                .join("\n")
        }
    }
}

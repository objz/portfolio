// src/commands/processor.rs

use std::{future::Future, pin::Pin};

use crate::{ascii, commands::system, terminal::renderer::TerminalRenderer};

use super::{commands, misc};

/// A command’s result can either be immediate text,
/// or an animated async routine (no Send bound on the future).
pub enum CommandResult {
    Output(String),

    /// Boxed Fn so we can capture owned data in an async block.
    Animated(
        Box<dyn Fn(TerminalRenderer) -> Pin<Box<dyn Future<Output = ()> + 'static>> + 'static>,
    ),
}

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

    pub fn get_working_dir(&self) -> String {
        commands::pwd(&[])
    }

    /// Handle one line of input, returning either
    /// immediate `Output` or an `Animated` future, plus
    /// whether `cd` was invoked.
    pub fn handle(&mut self, input: &str) -> (CommandResult, bool) {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return (CommandResult::Output(String::new()), false);
        }

        self.history.push(trimmed.to_string());

        // Split into &str parts...
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        let cmd = parts[0];

        // ...then own them as Strings so we can later move them into async.
        let args_owned: Vec<String> = parts.iter().skip(1).map(|s| s.to_string()).collect();

        // A helper slice for all the immediate (non-animated) commands:
        let args: Vec<&str> = args_owned.iter().map(String::as_str).collect();

        let directory_changed = cmd == "cd";

        let result = match cmd {
            // --- System built-ins
            "clear" => CommandResult::Output(system::clear(&args)),
            "history" => CommandResult::Output(self.print_history(&args)),
            "echo" => CommandResult::Output(system::echo(&args)),
            "date" => CommandResult::Output(system::date(&args)),
            "uptime" => CommandResult::Output(system::uptime(&args)),
            "neofetch" => CommandResult::Output(system::neofetch(&args)),

            // --- File-system commands
            "ls" => CommandResult::Output(commands::ls(&args)),
            "cd" => CommandResult::Output(commands::cd(&args)),
            "cat" => CommandResult::Output(commands::cat(&args)),
            "pwd" => CommandResult::Output(commands::pwd(&args)),
            "tree" => CommandResult::Output(commands::tree(&args)),
            "mkdir" => CommandResult::Output(commands::mkdir(&args)),
            "touch" => CommandResult::Output(commands::touch(&args)),
            "rm" => CommandResult::Output(commands::rm(&args)),
            "uname" => CommandResult::Output(commands::uname(&args)),
            "ln" => CommandResult::Output(commands::ln(&args)),
            "ll" => CommandResult::Output(commands::ls(&["-la"])),

            // --- Miscellany
            "help" => CommandResult::Output(misc::help(&args)),
            "sudo" => CommandResult::Output(misc::sudo(&args)),
            "cowsay" => CommandResult::Output(misc::cowsay(&args)),
            "lolcat" => CommandResult::Output(misc::lolcat(&args)),
            "calc" => CommandResult::Output(misc::calc(&args)),

            // in src/commands/processor.rs, inside your match arm for "sl":
            "sl" => {
                // own the args so we can clone again per invocation
                let args_clone = args_owned.clone();
                CommandResult::Animated(Box::new(move |renderer: TerminalRenderer| {
                    // clone here instead of moving out of args_clone
                    let args_for_future = args_clone.clone();
                    Box::pin(async move {
                        // rebuild &str slices from the cloned Vec<String>
                        let arg_slices: Vec<&str> =
                            args_for_future.iter().map(String::as_str).collect();
                        let _ = ascii::sl::animate(&renderer, &arg_slices).await;
                    })
                }))
            }

            // --- Unknown
            _ => CommandResult::Output(format!("zsh: command not found: {}", cmd)),
        };

        (result, directory_changed)
    }

    fn print_history(&self, _args: &[&str]) -> String {
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

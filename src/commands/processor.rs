use std::{future::Future, pin::Pin};

use crate::{
    ascii,
    commands::system,
    github,
    shell::{self, ExecutionCondition},
    terminal::renderer::{LineOptions, TerminalRenderer},
};

use super::{commands, filesystem, misc};

pub enum CommandResult {
    Output(String),
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

    pub fn handle(&mut self, input: &str) -> (CommandResult, bool) {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return (CommandResult::Output(String::new()), false);
        }

        self.history.push(trimmed.to_string());

        let parsed_segments = match shell::parse_line(trimmed) {
            Ok(parsed) => parsed,
            Err(error) => {
                return (CommandResult::Output(format!("shell: {}", error)), false);
            }
        };

        if parsed_segments.is_empty() {
            return (CommandResult::Output(String::new()), false);
        }

        if parsed_segments.len() == 1 {
            let segment = &parsed_segments[0];
            if segment.pipeline.len() == 1 && segment.redirection.is_none() {
                let tokens = &segment.pipeline[0];
                let (result, success, directory_changed) = self.execute_single(tokens, None);
                let directory_changed = directory_changed && success;
                return (result, directory_changed);
            }
        }

        let mut outputs = Vec::new();
        let mut last_success = true;
        let mut directory_changed = false;

        for segment in parsed_segments {
            if segment.condition == ExecutionCondition::OnSuccess && !last_success {
                continue;
            }

            let (mut output, success, changed_dir) = self.execute_pipeline(&segment.pipeline);
            directory_changed |= changed_dir && success;
            last_success = success;

            if let Some(redirection) = segment.redirection {
                match commands::write_output(&redirection.path, &output, redirection.append) {
                    Ok(()) => output.clear(),
                    Err(error) => {
                        output = error;
                        last_success = false;
                    }
                }
            }

            if !output.is_empty() {
                outputs.push(output);
            }
        }

        (CommandResult::Output(outputs.join("\n")), directory_changed)
    }

    fn execute_pipeline(&self, pipeline: &[Vec<String>]) -> (String, bool, bool) {
        let mut piped_input: Option<String> = None;
        let mut directory_changed = false;
        let mut last_success = true;

        for tokens in pipeline {
            let (result, success, changed_dir) =
                self.execute_single(tokens, piped_input.as_deref());
            directory_changed |= changed_dir && success;
            last_success = success;

            match result {
                CommandResult::Output(output) => {
                    piped_input = Some(output);
                }
                CommandResult::Animated(_) => {
                    return (
                        "shell: animated commands cannot be piped; run them directly".to_string(),
                        false,
                        directory_changed,
                    );
                }
            }

            if !last_success {
                break;
            }
        }

        (
            piped_input.unwrap_or_default(),
            last_success,
            directory_changed,
        )
    }

    fn execute_single(
        &self,
        tokens: &[String],
        stdin: Option<&str>,
    ) -> (CommandResult, bool, bool) {
        let cmd = tokens.first().map(String::as_str).unwrap_or_default();
        let args_owned: Vec<String> = tokens.iter().skip(1).cloned().collect();
        let stdin_owned = stdin
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);

        let args_or_stdin: Vec<String> = if args_owned.is_empty() {
            stdin_owned.clone().into_iter().collect()
        } else {
            args_owned.clone()
        };

        let args: Vec<&str> = args_owned.iter().map(String::as_str).collect();
        let args_with_stdin: Vec<&str> = args_or_stdin.iter().map(String::as_str).collect();

        let directory_changed = cmd == "cd";

        let result = match cmd {
            "clear" => CommandResult::Output(system::clear(&args)),
            "history" => CommandResult::Output(self.print_history(&args)),
            "echo" => {
                if args.is_empty() {
                    CommandResult::Output(stdin_owned.clone().unwrap_or_default())
                } else {
                    CommandResult::Output(system::echo(&args))
                }
            }
            "date" => CommandResult::Output(system::date(&args)),
            "uptime" => CommandResult::Output(system::uptime(&args)),
            "neofetch" => CommandResult::Output(system::neofetch(&args)),
            "hostname" => CommandResult::Output(system::hostname(&args)),
            "whoami" => CommandResult::Output(system::whoami(&args)),

            "ls" => CommandResult::Output(commands::ls(&args)),
            "cd" => CommandResult::Output(commands::cd(&args)),
            "cat" => {
                if args.is_empty() {
                    CommandResult::Output(stdin_owned.clone().unwrap_or_default())
                } else {
                    CommandResult::Output(commands::cat(&args))
                }
            }
            "pwd" => CommandResult::Output(commands::pwd(&args)),
            "tree" => CommandResult::Output(commands::tree(&args)),
            "mkdir" => CommandResult::Output(commands::mkdir(&args)),
            "touch" => CommandResult::Output(commands::touch(&args)),
            "rm" => CommandResult::Output(commands::rm(&args)),
            "uname" => CommandResult::Output(commands::uname(&args)),
            "ln" => CommandResult::Output(commands::ln(&args)),
            "cp" => CommandResult::Output(commands::cp(&args)),
            "mv" => CommandResult::Output(commands::mv(&args)),
            "nvim" => CommandResult::Output(commands::nvim(&args)),
            "ll" => CommandResult::Output(commands::ls(&["-la"])),

            "help" => CommandResult::Output(misc::help(&args)),
            "sudo" => CommandResult::Output(misc::sudo(&args)),
            "cowsay" => CommandResult::Output(misc::cowsay(&args_with_stdin)),
            "lolcat" => CommandResult::Output(misc::lolcat(&args_with_stdin)),
            "calc" => CommandResult::Output(misc::calc(&args)),

            "sl" => {
                let args_clone = args_owned.clone();
                CommandResult::Animated(Box::new(move |renderer: TerminalRenderer| {
                    let args_for_future = args_clone.clone();
                    Box::pin(async move {
                        let arg_slices: Vec<&str> =
                            args_for_future.iter().map(String::as_str).collect();
                        let _ = ascii::sl::animate(&renderer, &arg_slices).await;
                    })
                }))
            }

            "project" => {
                if args_owned.is_empty() {
                    CommandResult::Output("Usage: project <repo> [username]".to_string())
                } else {
                    let repo = args_owned[0].clone();
                    let username = args_owned
                        .get(1)
                        .cloned()
                        .unwrap_or_else(|| github::DEFAULT_GITHUB_USER.to_string());

                    CommandResult::Animated(Box::new(move |renderer: TerminalRenderer| {
                        let repo = repo.clone();
                        let username = username.clone();

                        Box::pin(async move {
                            let normalized_repo = Self::sanitize_github_segment(&repo);
                            let normalized_user = Self::sanitize_github_segment(&username);

                            if normalized_repo.is_empty() {
                                renderer
                                    .add_line(
                                        "project: invalid repository name",
                                        Some(LineOptions::new().with_color("error")),
                                    )
                                    .await;
                                return;
                            }

                            let user = if normalized_user.is_empty() {
                                github::DEFAULT_GITHUB_USER.to_string()
                            } else {
                                normalized_user
                            };

                            renderer
                                .add_line(
                                    &format!(
                                        "Fetching project metadata for {}/{}...",
                                        user, normalized_repo
                                    ),
                                    Some(LineOptions::new().with_color("cyan")),
                                )
                                .await;

                            match github::fetch_projects_cached(&user, false).await {
                                Ok(snapshot) => {
                                    Self::render_cache_meta(
                                        &renderer,
                                        snapshot.state,
                                        snapshot.fetched_at,
                                    )
                                    .await;

                                    let project = snapshot.projects.iter().find(|entry| {
                                        entry.name.eq_ignore_ascii_case(&normalized_repo)
                                    });

                                    if let Some(project) = project {
                                        let language = project.language.as_deref().unwrap_or("n/a");
                                        let description = project
                                            .description
                                            .as_deref()
                                            .unwrap_or("No description available.");
                                        let updated_at = Self::short_iso_date(&project.updated_at);

                                        renderer
                                            .add_line(
                                                &format!("Project: {}", project.name),
                                                Some(LineOptions::new().with_color("success")),
                                            )
                                            .await;
                                        renderer
                                            .add_line(
                                                &format!(
                                                    "Language: {} | stars:{} | updated:{}",
                                                    language, project.stars, updated_at
                                                ),
                                                None,
                                            )
                                            .await;
                                        renderer
                                            .add_line(
                                                &format!("URL: {}", project.html_url),
                                                Some(LineOptions::new().with_color("cyan")),
                                            )
                                            .await;
                                        renderer.add_line(description, None).await;
                                        renderer
                                            .add_line(
                                                &format!(
                                                    "Next: readme {} {} | open {} {}",
                                                    project.name, user, project.name, user
                                                ),
                                                Some(LineOptions::new().with_color("warning")),
                                            )
                                            .await;
                                    } else {
                                        renderer
                                            .add_line(
                                                &format!(
                                                    "project: '{}' was not found for {}",
                                                    normalized_repo, user
                                                ),
                                                Some(LineOptions::new().with_color("error")),
                                            )
                                            .await;
                                    }
                                }
                                Err(error) => {
                                    renderer
                                        .add_line(
                                            &format!("project: {}", error),
                                            Some(LineOptions::new().with_color("error")),
                                        )
                                        .await;
                                }
                            }
                        })
                    }))
                }
            }

            "readme" => {
                if args_owned.is_empty() {
                    CommandResult::Output("Usage: readme <repo> [username]".to_string())
                } else {
                    let repo = args_owned[0].clone();
                    let username = args_owned
                        .get(1)
                        .cloned()
                        .unwrap_or_else(|| github::DEFAULT_GITHUB_USER.to_string());

                    CommandResult::Animated(Box::new(move |renderer: TerminalRenderer| {
                        let repo = repo.clone();
                        let username = username.clone();

                        Box::pin(async move {
                            let normalized_repo = Self::sanitize_github_segment(&repo);
                            let normalized_user = Self::sanitize_github_segment(&username);

                            if normalized_repo.is_empty() {
                                renderer
                                    .add_line(
                                        "readme: invalid repository name",
                                        Some(LineOptions::new().with_color("error")),
                                    )
                                    .await;
                                return;
                            }

                            let user = if normalized_user.is_empty() {
                                github::DEFAULT_GITHUB_USER.to_string()
                            } else {
                                normalized_user
                            };

                            renderer
                                .add_line(
                                    &format!("Fetching README for {}/{}...", user, normalized_repo),
                                    Some(LineOptions::new().with_color("cyan")),
                                )
                                .await;

                            match github::fetch_readme_cached(&user, &normalized_repo, false).await
                            {
                                Ok(snapshot) => {
                                    Self::render_readme(&renderer, &snapshot).await;
                                }
                                Err(error) => {
                                    renderer
                                        .add_line(
                                            &format!("readme: {}", error),
                                            Some(LineOptions::new().with_color("error")),
                                        )
                                        .await;
                                }
                            }
                        })
                    }))
                }
            }

            "open" => {
                if args_owned.is_empty() {
                    CommandResult::Output("Usage: open <repo|url> [username]".to_string())
                } else {
                    let target = args_owned[0].clone();
                    let explicit_user = args_owned.get(1).cloned();

                    let url = if target.starts_with("http://") || target.starts_with("https://") {
                        target
                    } else if let Some(project_url) = filesystem::project_url_for_token(&target) {
                        project_url
                    } else {
                        let repo = Self::sanitize_github_segment(&target);
                        let user = explicit_user
                            .map(|name| Self::sanitize_github_segment(&name))
                            .filter(|name| !name.is_empty())
                            .unwrap_or_else(|| github::DEFAULT_GITHUB_USER.to_string());

                        if repo.is_empty() {
                            return (
                                CommandResult::Output("open: invalid repository name".to_string()),
                                false,
                                directory_changed,
                            );
                        }

                        format!("https://github.com/{}/{}", user, repo)
                    };

                    if let Some(window) = web_sys::window() {
                        let _ = window.open_with_url_and_target(&url, "_blank");
                        CommandResult::Output(format!("Opened {}", url))
                    } else {
                        CommandResult::Output("open: browser window unavailable".to_string())
                    }
                }
            }

            _ => CommandResult::Output(format!("zsh: command not found: {}", cmd)),
        };

        let success = match &result {
            CommandResult::Animated(_) => true,
            CommandResult::Output(output) => Self::output_success(cmd, output),
        };

        (result, success, directory_changed)
    }

    fn output_success(cmd: &str, output: &str) -> bool {
        if output.is_empty() {
            return true;
        }

        if output.starts_with("zsh: command not found") {
            return false;
        }

        if output.starts_with("Usage:") || output.starts_with("Error:") {
            return false;
        }

        if output.starts_with("sudo: access denied") {
            return false;
        }

        if cmd == "ll" {
            return !output.starts_with("ls:");
        }

        let error_prefix = format!("{}:", cmd);
        !output.starts_with(&error_prefix)
    }

    fn print_history(&self, _args: &[&str]) -> String {
        let options = match crate::commands::options::parse(
            "history",
            _args,
            crate::commands::options::OptionSpec::new(&[], &["help"]),
        ) {
            Ok(options) => options,
            Err(error) => return error,
        };

        if options.has_help() {
            return "Usage: history".to_string();
        }

        if let Err(error) = crate::commands::options::no_args("history", &options.operands) {
            return error;
        }

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

    pub async fn sync_default_projects(force_refresh: bool) -> Result<usize, String> {
        Self::sync_projects_for_user(github::DEFAULT_GITHUB_USER, force_refresh).await
    }

    pub async fn sync_projects_for_user(
        username: &str,
        force_refresh: bool,
    ) -> Result<usize, String> {
        let normalized_user = Self::sanitize_github_segment(username);
        let user = if normalized_user.is_empty() {
            github::DEFAULT_GITHUB_USER.to_string()
        } else {
            normalized_user
        };

        let snapshot = github::fetch_projects_cached(&user, force_refresh).await?;
        Self::sync_projects_directory(&snapshot, force_refresh).await
    }

    fn sanitize_github_segment(value: &str) -> String {
        value
            .trim()
            .trim_matches('/')
            .chars()
            .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
            .collect()
    }

    fn short_iso_date(iso: &str) -> String {
        iso.chars().take(10).collect()
    }

    async fn sync_projects_directory(
        snapshot: &github::ProjectsSnapshot,
        force_refresh: bool,
    ) -> Result<usize, String> {
        let mut files = Vec::new();

        for project in &snapshot.projects {
            let readme_snapshot =
                github::fetch_readme_cached(&snapshot.username, &project.name, force_refresh).await;

            let repo_url = project.html_url.clone();
            let readme_section = match readme_snapshot {
                Ok(readme) => github::render_markdown_to_terminal(&readme.content),
                Err(error) => format!(
                    "README could not be loaded right now: {}\n\nRetry later to refresh project files.",
                    error
                ),
            };

            let language = project.language.as_deref().unwrap_or("n/a");
            let description = project
                .description
                .as_deref()
                .unwrap_or("No description provided.")
                .replace('\n', " ");
            let updated_at = Self::short_iso_date(&project.updated_at);

            let content = format!(
                "# {}\n\nRepository: {}\nUpdated: {}\nLanguage: {}\nStars: {}\n\nDescription: {}\n\n---\n\n{}",
                project.name,
                repo_url,
                updated_at,
                language,
                project.stars,
                description,
                readme_section
            );

            files.push(filesystem::ProjectFileSpec {
                file_name: format!("{}.md", project.name),
                repo_name: project.name.clone(),
                repo_url,
                content,
            });
        }

        filesystem::replace_projects_directory(&files)?;
        Ok(files.len())
    }

    async fn render_readme(renderer: &TerminalRenderer, snapshot: &github::ReadmeSnapshot) {
        let repo_url = format!("https://github.com/{}/{}", snapshot.username, snapshot.repo);

        renderer
            .add_line(
                &format!("README for {}/{}", snapshot.username, snapshot.repo),
                Some(LineOptions::new().with_color("success")),
            )
            .await;

        renderer
            .add_line(
                &format!("Repository: {}", repo_url),
                Some(LineOptions::new().with_color("cyan")),
            )
            .await;

        Self::render_cache_meta(renderer, snapshot.state, snapshot.fetched_at).await;

        let max_lines = 220usize;
        let rendered = github::render_markdown_to_terminal(&snapshot.content);
        let lines: Vec<&str> = rendered.lines().collect();

        for line in lines.iter().take(max_lines) {
            renderer.add_line(line, None).await;
        }

        if lines.len() > max_lines {
            renderer
                .add_line(
                    &format!(
                        "... README truncated: showing {} of {} lines.",
                        max_lines,
                        lines.len()
                    ),
                    Some(LineOptions::new().with_color("warning")),
                )
                .await;
        }
    }

    async fn render_cache_meta(
        renderer: &TerminalRenderer,
        state: github::CacheState,
        fetched_at: f64,
    ) {
        let age = github::format_age(fetched_at);
        let (label, color) = match state {
            github::CacheState::Live => ("source: live", "cyan"),
            github::CacheState::Cached => ("source: cached", "gray"),
            github::CacheState::StaleFallback => ("source: stale cache", "warning"),
        };

        renderer
            .add_line(
                &format!("{} | fetched {}", label, age),
                Some(LineOptions::new().with_color(color)),
            )
            .await;
    }
}

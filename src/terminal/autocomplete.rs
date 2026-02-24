use crate::commands::registry::{self, PathCompletionMode};

#[derive(Default)]
pub struct AutoComplete;

#[derive(Debug, Clone)]
pub struct CompletionEdit {
    pub input: String,
    pub cursor: usize,
}

#[derive(Debug)]
pub enum CompletionResult {
    None,
    Single(CompletionEdit),
    Multiple {
        options: Vec<String>,
        common: Option<CompletionEdit>,
    },
}

#[derive(Debug)]
struct CompletionContext {
    prefix: String,
    suffix: String,
    token_match: String,
    quote_prefix: Option<char>,
    command_position: bool,
    command_name: Option<String>,
}

impl CompletionContext {
    fn from_input(input: &str, cursor: usize) -> Self {
        let cursor = clamp_to_char_boundary(input, cursor);
        let before_cursor = &input[..cursor];
        let suffix = input[cursor..].to_string();

        let segment_start = find_active_segment_start(before_cursor);
        let segment = &before_cursor[segment_start..];

        let token_start_in_segment = find_token_start(segment);
        let token_start = segment_start + token_start_in_segment;
        let prefix = before_cursor[..token_start].to_string();
        let current_token = before_cursor[token_start..].to_string();

        let (quote_prefix, token_match) = strip_leading_quote(&current_token);

        let command_prefix = segment[..token_start_in_segment].trim_end();
        let parsed_prefix = parse_shell_tokens(command_prefix);
        let command_position = parsed_prefix.is_empty();
        let command_name = parsed_prefix.first().cloned();

        Self {
            prefix,
            suffix,
            token_match,
            quote_prefix,
            command_position,
            command_name,
        }
    }
}

fn find_active_segment_start(input: &str) -> usize {
    let mut start = 0usize;
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;
    let mut chars = input.char_indices().peekable();

    while let Some((idx, ch)) = chars.next() {
        if escaped {
            escaped = false;
            continue;
        }

        match ch {
            '\\' if !in_single => escaped = true,
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            ';' if !in_single && !in_double => {
                start = idx + 1;
            }
            '|' if !in_single && !in_double => {
                // Single pipe (not ||)
                if chars.peek().map(|(_, c)| *c) != Some('|') {
                    start = idx + 1;
                }
            }
            '&' if !in_single && !in_double => {
                if let Some((next_idx, '&')) = chars.peek().copied() {
                    let _ = chars.next();
                    start = next_idx + 1;
                }
            }
            _ => {}
        }
    }

    start
}

impl AutoComplete {
    pub fn new() -> Self {
        Self
    }

    pub fn complete(
        &mut self,
        input: &str,
        cursor_pos: usize,
        current_path: &[String],
    ) -> CompletionResult {
        if input.trim().is_empty() {
            return CompletionResult::None;
        }

        let context = CompletionContext::from_input(input, cursor_pos);

        if context.command_position {
            return self.complete_command(&context);
        }

        let Some(command_name) = context.command_name.as_deref() else {
            return CompletionResult::None;
        };

        match registry::path_completion_mode(command_name) {
            Some(PathCompletionMode::DirectoriesOnly) => {
                self.complete_path(&context, current_path, true)
            }
            Some(PathCompletionMode::FilesAndDirectories) => {
                self.complete_path(&context, current_path, false)
            }
            Some(PathCompletionMode::ProjectRepos) => self.complete_project_repo(&context),
            None => CompletionResult::None,
        }
    }

    fn complete_project_repo(&self, context: &CompletionContext) -> CompletionResult {
        use crate::commands::filesystem::project_repo_candidates;

        let needle = context.token_match.to_lowercase();
        let mut matches: Vec<String> = project_repo_candidates()
            .into_iter()
            .filter(|candidate| candidate.to_lowercase().starts_with(&needle))
            .collect();

        if matches.is_empty() {
            return CompletionResult::None;
        }

        matches.dedup();

        match matches.len() {
            1 => CompletionResult::Single(self.apply_completion(context, &matches[0], true)),
            _ => {
                let common = find_common_prefix(&matches).and_then(|prefix| {
                    if prefix.len() > context.token_match.len() {
                        Some(self.apply_completion(context, &prefix, false))
                    } else {
                        None
                    }
                });

                CompletionResult::Multiple {
                    options: matches,
                    common,
                }
            }
        }
    }

    fn complete_command(&self, context: &CompletionContext) -> CompletionResult {
        let mut matches: Vec<String> = registry::command_names()
            .into_iter()
            .filter(|name| name.starts_with(&context.token_match))
            .collect();

        if matches.is_empty() {
            return CompletionResult::None;
        }

        matches.sort();
        matches.dedup();

        match matches.len() {
            1 => CompletionResult::Single(self.apply_completion(context, &matches[0], true)),
            _ => {
                let common = find_common_prefix(&matches).and_then(|prefix| {
                    if prefix.len() > context.token_match.len() {
                        Some(self.apply_completion(context, &prefix, false))
                    } else {
                        None
                    }
                });

                CompletionResult::Multiple {
                    options: matches,
                    common,
                }
            }
        }
    }

    fn complete_path(
        &self,
        context: &CompletionContext,
        current_path: &[String],
        dirs_only: bool,
    ) -> CompletionResult {
        use crate::commands::filesystem::{autocomplete_entries, normalize_path};

        let partial = context.token_match.as_str();

        let (dir_path, filename_prefix) = if let Some(last_slash) = partial.rfind('/') {
            (&partial[..last_slash + 1], &partial[last_slash + 1..])
        } else {
            ("", partial)
        };

        let search_path = if dir_path.is_empty() {
            current_path.to_vec()
        } else {
            normalize_path(dir_path, current_path)
        };

        let mut matches: Vec<String> = autocomplete_entries(&search_path, dirs_only)
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

        if matches.is_empty() {
            return CompletionResult::None;
        }

        matches.sort();
        matches.dedup();

        match matches.len() {
            1 => {
                let candidate = &matches[0];
                let append_space = !candidate.ends_with('/');
                CompletionResult::Single(self.apply_completion(context, candidate, append_space))
            }
            _ => {
                let common = find_common_prefix(&matches).and_then(|prefix| {
                    if prefix.len() > context.token_match.len() {
                        Some(self.apply_completion(context, &prefix, false))
                    } else {
                        None
                    }
                });

                CompletionResult::Multiple {
                    options: matches,
                    common,
                }
            }
        }
    }

    fn apply_completion(
        &self,
        context: &CompletionContext,
        candidate: &str,
        append_space: bool,
    ) -> CompletionEdit {
        let escaped_candidate = if context.quote_prefix.is_none() {
            escape_if_needed(candidate)
        } else {
            candidate.to_string()
        };

        let mut replacement = match context.quote_prefix {
            Some(quote) => format!("{}{}", quote, escaped_candidate),
            None => escaped_candidate,
        };

        if append_space && context.suffix.is_empty() {
            replacement.push(' ');
        }

        let cursor = context.prefix.len() + replacement.len();
        let input = format!("{}{}{}", context.prefix, replacement, context.suffix);

        CompletionEdit { input, cursor }
    }
}

fn parse_shell_tokens(input: &str) -> Vec<String> {
    if input.trim().is_empty() {
        return Vec::new();
    }

    shell_words::split(input).unwrap_or_else(|_| {
        input
            .split_whitespace()
            .map(|segment| segment.to_string())
            .collect()
    })
}

fn strip_leading_quote(token: &str) -> (Option<char>, String) {
    if let Some(first) = token.chars().next() {
        if first == '\'' || first == '"' {
            let quote_len = first.len_utf8();
            return (Some(first), token[quote_len..].to_string());
        }
    }

    (None, token.to_string())
}

fn escape_if_needed(value: &str) -> String {
    if value.chars().any(char::is_whitespace) {
        value
            .chars()
            .flat_map(|ch| {
                if ch.is_whitespace() {
                    ['\\', ch]
                } else {
                    ['\0', ch]
                }
            })
            .filter(|ch| *ch != '\0')
            .collect()
    } else {
        value.to_string()
    }
}

fn find_token_start(input: &str) -> usize {
    let mut start = 0;
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;

    let mut chars = input.char_indices().peekable();
    while let Some((idx, ch)) = chars.next() {
        if escaped {
            escaped = false;
            continue;
        }

        match ch {
            '\\' if !in_single => escaped = true,
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            _ if ch.is_whitespace() && !in_single && !in_double => {
                start = idx + ch.len_utf8();
            }
            ';' if !in_single && !in_double => {
                start = idx + 1;
            }
            '|' if !in_single && !in_double => {
                // Single pipe (not ||)
                if chars.peek().map(|(_, c)| *c) != Some('|') {
                    start = idx + 1;
                }
            }
            '&' if !in_single && !in_double => {
                if let Some((next_idx, '&')) = chars.peek().copied() {
                    let _ = chars.next();
                    start = next_idx + 1;
                }
            }
            _ => {}
        }
    }

    start
}

fn clamp_to_char_boundary(input: &str, cursor: usize) -> usize {
    let mut index = cursor.min(input.len());
    while index > 0 && !input.is_char_boundary(index) {
        index -= 1;
    }
    index
}

pub fn find_common_prefix(strings: &[String]) -> Option<String> {
    if strings.is_empty() {
        return None;
    }

    let mut prefix = strings[0].clone();

    for item in strings.iter().skip(1) {
        while !item.starts_with(&prefix) {
            if prefix.is_empty() {
                return None;
            }
            prefix.pop();
        }
    }

    if prefix.is_empty() {
        None
    } else {
        Some(prefix)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExecutionCondition {
    Always,
    OnSuccess,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Redirection {
    pub path: String,
    pub append: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Segment {
    pub condition: ExecutionCondition,
    pub pipeline: Vec<Vec<String>>,
    pub redirection: Option<Redirection>,
}

pub fn parse_line(input: &str) -> Result<Vec<Segment>, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    let mut parsed = Vec::new();
    for (condition, raw_segment) in split_chain_segments(trimmed) {
        let segment_text = raw_segment.trim();
        if segment_text.is_empty() {
            continue;
        }

        let (command_text, redirection) = split_redirection(segment_text)?;
        let pipeline = split_pipeline(&command_text)?;

        parsed.push(Segment {
            condition,
            pipeline,
            redirection,
        });
    }

    Ok(parsed)
}

fn split_chain_segments(input: &str) -> Vec<(ExecutionCondition, String)> {
    let mut segments = Vec::new();
    let mut current = String::new();
    let mut chars = input.chars().peekable();

    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;
    let mut next_condition = ExecutionCondition::Always;

    let push_segment = |segments: &mut Vec<(ExecutionCondition, String)>,
                        condition: ExecutionCondition,
                        raw: &str| {
        let segment = raw.trim();
        if !segment.is_empty() {
            segments.push((condition, segment.to_string()));
        }
    };

    while let Some(ch) = chars.next() {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }

        match ch {
            '\\' => {
                current.push(ch);
                escaped = true;
            }
            '\'' if !in_double => {
                in_single = !in_single;
                current.push(ch);
            }
            '"' if !in_single => {
                in_double = !in_double;
                current.push(ch);
            }
            ';' if !in_single && !in_double => {
                push_segment(&mut segments, next_condition, &current);
                current.clear();
                next_condition = ExecutionCondition::Always;
            }
            '&' if !in_single && !in_double => {
                if chars.peek() == Some(&'&') {
                    chars.next();
                    push_segment(&mut segments, next_condition, &current);
                    current.clear();
                    next_condition = ExecutionCondition::OnSuccess;
                } else {
                    current.push(ch);
                }
            }
            _ => current.push(ch),
        }
    }

    push_segment(&mut segments, next_condition, &current);
    segments
}

fn split_redirection(segment: &str) -> Result<(String, Option<Redirection>), String> {
    let mut chars = segment.char_indices().peekable();
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;
    let mut redirection: Option<(usize, usize, bool)> = None;

    while let Some((idx, ch)) = chars.next() {
        if escaped {
            escaped = false;
            continue;
        }

        match ch {
            '\\' if !in_single => escaped = true,
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            '>' if !in_single && !in_double => {
                if let Some((next_idx, '>')) = chars.peek().copied() {
                    let _ = chars.next();
                    let op_len = (next_idx - idx) + 1;
                    redirection = Some((idx, op_len, true));
                } else {
                    redirection = Some((idx, ch.len_utf8(), false));
                }
            }
            _ => {}
        }
    }

    let Some((op_start, op_len, append)) = redirection else {
        return Ok((segment.trim().to_string(), None));
    };

    let command_part = segment[..op_start].trim();
    if command_part.is_empty() {
        return Err("missing command before redirection".into());
    }

    let path_part = segment[(op_start + op_len)..].trim();
    if path_part.is_empty() {
        return Err("missing redirection target".into());
    }

    let path_tokens = shell_words::split(path_part)
        .map_err(|error| format!("invalid redirect path: {}", error))?;

    if path_tokens.len() != 1 {
        return Err("redirection target must resolve to one path".into());
    }

    Ok((
        command_part.to_string(),
        Some(Redirection {
            path: path_tokens[0].clone(),
            append,
        }),
    ))
}

fn split_pipeline(segment: &str) -> Result<Vec<Vec<String>>, String> {
    let mut commands = Vec::new();
    let mut current = String::new();

    let mut chars = segment.chars().peekable();
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;

    let push_command = |commands: &mut Vec<Vec<String>>, raw: &str| -> Result<(), String> {
        let command = raw.trim();
        if command.is_empty() {
            return Err("empty command in pipeline".into());
        }

        let tokens =
            shell_words::split(command).map_err(|error| format!("shell parse error: {}", error))?;

        if tokens.is_empty() {
            return Err("empty command in pipeline".into());
        }

        commands.push(tokens);
        Ok(())
    };

    while let Some(ch) = chars.next() {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }

        match ch {
            '\\' => {
                current.push(ch);
                escaped = true;
            }
            '\'' if !in_double => {
                in_single = !in_single;
                current.push(ch);
            }
            '"' if !in_single => {
                in_double = !in_double;
                current.push(ch);
            }
            '|' if !in_single && !in_double => {
                push_command(&mut commands, &current)?;
                current.clear();
            }
            _ => current.push(ch),
        }
    }

    push_command(&mut commands, &current)?;
    Ok(commands)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_chain_and_pipeline() {
        let parsed = parse_line("echo hi | lolcat && pwd; ls").expect("parse should succeed");
        assert_eq!(parsed.len(), 3);

        assert_eq!(parsed[0].condition, ExecutionCondition::Always);
        assert_eq!(parsed[0].pipeline.len(), 2);
        assert_eq!(parsed[1].condition, ExecutionCondition::OnSuccess);
        assert_eq!(parsed[2].condition, ExecutionCondition::Always);
    }

    #[test]
    fn parses_redirection() {
        let parsed = parse_line("projects > repos.txt").expect("parse should succeed");
        assert_eq!(parsed.len(), 1);

        let redir = parsed[0].redirection.as_ref().expect("redirection missing");
        assert_eq!(redir.path, "repos.txt");
        assert!(!redir.append);
    }

    #[test]
    fn preserves_pipes_inside_quotes() {
        let parsed = parse_line("echo 'a|b' | lolcat").expect("parse should succeed");
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].pipeline.len(), 2);
        assert_eq!(parsed[0].pipeline[0][1], "a|b");
    }
}

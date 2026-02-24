use std::cell::RefCell;

use crate::commands::filesystem::{
    get_node, get_node_mut, normalize_path, Node, CURRENT_PATH, CURRENT_USER, FILESYSTEM,
};
use crate::terminal::buffer::{self, LineType};
use crate::terminal::renderer::TerminalRenderer;
use web_sys::{window, KeyboardEvent};

#[derive(Clone, Copy, PartialEq, Eq)]
enum EditorMode {
    Normal,
    Insert,
    Command,
}

#[derive(Clone)]
struct EditorState {
    path: Vec<String>,
    display_path: String,
    lines: Vec<String>,
    cursor_row: usize,
    cursor_col: usize,
    scroll_top: usize,
    mode: EditorMode,
    command_line: String,
    message: String,
    dirty: bool,
}

pub enum EditorEvent {
    Continue,
    Exit { message: Option<String> },
}

enum KeyAction {
    Continue,
    Exit(Option<String>),
}

thread_local! {
    static EDITOR: RefCell<Option<EditorState>> = const { RefCell::new(None) };
}

pub fn is_active() -> bool {
    EDITOR.with(|state| state.borrow().is_some())
}

pub fn open(target: &str) -> Result<(), String> {
    let cwd = CURRENT_PATH.lock().unwrap().clone();
    let resolved = normalize_path(target, &cwd);

    if resolved.is_empty() {
        return Err("nvim: invalid path".to_string());
    }

    let parent_path = &resolved[..resolved.len() - 1];

    let mut initial_lines = vec![String::new()];
    {
        let filesystem = FILESYSTEM.lock().unwrap();

        match get_node(&filesystem, &resolved) {
            Some(Node::File { content, .. }) => {
                initial_lines = if content.is_empty() {
                    vec![String::new()]
                } else {
                    content.lines().map(str::to_string).collect()
                };
            }
            Some(Node::Directory { .. }) => {
                return Err(format!("nvim: {}: is a directory", target));
            }
            Some(Node::Symlink { .. }) => {
                return Err(format!(
                    "nvim: {}: symbolic links are not supported",
                    target
                ));
            }
            None => match get_node(&filesystem, parent_path) {
                Some(Node::Directory { .. }) => {}
                Some(_) => {
                    return Err(format!("nvim: {}: parent is not a directory", target));
                }
                None => {
                    return Err(format!("nvim: {}: no such file or directory", target));
                }
            },
        }
    }

    let display_path = format!("/{}", resolved.join("/"));

    EDITOR.with(|state| {
        *state.borrow_mut() = Some(EditorState {
            path: resolved,
            display_path,
            lines: initial_lines,
            cursor_row: 0,
            cursor_col: 0,
            scroll_top: 0,
            mode: EditorMode::Normal,
            command_line: String::new(),
            message: "i: insert | :w write | :q quit | :wq write+quit".to_string(),
            dirty: false,
        });
    });

    set_editor_active_class(true);
    buffer::set_input_mode(buffer::InputMode::Disabled);
    Ok(())
}

pub fn handle_key(event: &KeyboardEvent) -> EditorEvent {
    let mut action = KeyAction::Continue;

    EDITOR.with(|slot| {
        let mut state_slot = slot.borrow_mut();
        let Some(state) = state_slot.as_mut() else {
            return;
        };

        if event.ctrl_key() && event.key().eq_ignore_ascii_case("c") {
            action = KeyAction::Exit(Some("nvim: canceled".to_string()));
        } else {
            action = match state.mode {
                EditorMode::Normal => handle_normal_mode(state, event),
                EditorMode::Insert => handle_insert_mode(state, event),
                EditorMode::Command => handle_command_mode(state, event),
            };
        }

        if matches!(action, KeyAction::Exit(_)) {
            set_editor_active_class(false);
            *state_slot = None;
        }
    });

    match action {
        KeyAction::Continue => EditorEvent::Continue,
        KeyAction::Exit(message) => EditorEvent::Exit { message },
    }
}

fn set_editor_active_class(active: bool) {
    let Some(window) = window() else {
        return;
    };

    let Some(document) = window.document() else {
        return;
    };

    let Some(body) = document.body() else {
        return;
    };

    let current = body.class_name();
    let mut classes: Vec<&str> = current.split_whitespace().collect();
    let has_flag = classes.iter().any(|name| *name == "editor-active");

    if active && !has_flag {
        classes.push("editor-active");
    } else if !active && has_flag {
        classes.retain(|name| *name != "editor-active");
    }

    body.set_class_name(&classes.join(" "));
}

pub fn render(renderer: &TerminalRenderer) {
    EDITOR.with(|slot| {
        let mut state_slot = slot.borrow_mut();
        let Some(state) = state_slot.as_mut() else {
            return;
        };

        let max_visible = renderer.max_visible_lines().max(6);
        let content_rows = max_visible.saturating_sub(3);

        if state.cursor_row < state.scroll_top {
            state.scroll_top = state.cursor_row;
        }

        if state.cursor_row >= state.scroll_top + content_rows {
            state.scroll_top = state.cursor_row + 1 - content_rows;
        }

        buffer::clear_buffer();

        let dirty_mark = if state.dirty { " [+]" } else { "" };
        let mode_label = match state.mode {
            EditorMode::Normal => "NORMAL",
            EditorMode::Insert => "INSERT",
            EditorMode::Command => "COMMAND",
        };

        buffer::add_line(
            format!(
                "\x1b[1;36mNVIM\x1b[0m {}{}  -- {}",
                state.display_path, dirty_mark, mode_label
            ),
            LineType::Output,
            None,
        );

        for row in 0..content_rows {
            let file_row = state.scroll_top + row;
            if file_row >= state.lines.len() {
                buffer::add_line("\x1b[90m~\x1b[0m".to_string(), LineType::Output, None);
                continue;
            }

            let line_number = format!("\x1b[90m{:>4}\x1b[0m", file_row + 1);
            let marker = if file_row == state.cursor_row {
                "\x1b[1;32m>\x1b[0m"
            } else {
                " "
            };

            let content = if file_row == state.cursor_row {
                line_with_cursor(&state.lines[file_row], state.cursor_col)
            } else {
                state.lines[file_row].clone()
            };

            buffer::add_line(
                format!("{} {} {}", marker, line_number, content),
                LineType::Output,
                None,
            );
        }

        let status = if state.mode == EditorMode::Command {
            format!(":{}", state.command_line)
        } else {
            state.message.clone()
        };

        buffer::add_line(
            format!("\x1b[90m-- {} lines --\x1b[0m", state.lines.len()),
            LineType::Output,
            None,
        );
        buffer::add_line(status, LineType::Output, None);

        buffer::reset_scroll();
        renderer.render();
    });
}

fn handle_normal_mode(state: &mut EditorState, event: &KeyboardEvent) -> KeyAction {
    match event.key().as_str() {
        "h" | "ArrowLeft" => move_left(state),
        "j" | "ArrowDown" => move_down(state),
        "k" | "ArrowUp" => move_up(state),
        "l" | "ArrowRight" => move_right(state),
        "0" | "Home" => state.cursor_col = 0,
        "$" | "End" => state.cursor_col = char_len(&state.lines[state.cursor_row]),
        "i" => {
            state.mode = EditorMode::Insert;
            state.message = "-- INSERT --".to_string();
        }
        "a" => {
            move_right(state);
            state.mode = EditorMode::Insert;
            state.message = "-- INSERT --".to_string();
        }
        "o" => {
            let insert_at = state.cursor_row + 1;
            state.lines.insert(insert_at, String::new());
            state.cursor_row = insert_at;
            state.cursor_col = 0;
            state.mode = EditorMode::Insert;
            state.dirty = true;
            state.message = "-- INSERT --".to_string();
        }
        "x" => {
            delete_under_cursor(state);
        }
        ":" => {
            state.mode = EditorMode::Command;
            state.command_line.clear();
        }
        _ => {}
    }

    KeyAction::Continue
}

fn handle_insert_mode(state: &mut EditorState, event: &KeyboardEvent) -> KeyAction {
    match event.key().as_str() {
        "Escape" => {
            state.mode = EditorMode::Normal;
            state.message.clear();
        }
        "ArrowLeft" => move_left(state),
        "ArrowDown" => move_down(state),
        "ArrowUp" => move_up(state),
        "ArrowRight" => move_right(state),
        "Home" => state.cursor_col = 0,
        "End" => state.cursor_col = char_len(&state.lines[state.cursor_row]),
        "Enter" => split_line(state),
        "Backspace" => backspace(state),
        "Tab" => {
            for _ in 0..4 {
                insert_char(state, ' ');
            }
        }
        key if key.chars().count() == 1 && !event.ctrl_key() && !event.meta_key() => {
            if let Some(ch) = key.chars().next() {
                insert_char(state, ch);
            }
        }
        _ => {}
    }

    KeyAction::Continue
}

fn handle_command_mode(state: &mut EditorState, event: &KeyboardEvent) -> KeyAction {
    match event.key().as_str() {
        "Escape" => {
            state.mode = EditorMode::Normal;
            state.command_line.clear();
            state.message.clear();
            KeyAction::Continue
        }
        "Enter" => {
            let command = state.command_line.trim().to_string();
            state.command_line.clear();
            execute_command(state, &command)
        }
        "Backspace" => {
            state.command_line.pop();
            KeyAction::Continue
        }
        key if key.chars().count() == 1 && !event.ctrl_key() && !event.meta_key() => {
            state.command_line.push_str(key);
            KeyAction::Continue
        }
        _ => KeyAction::Continue,
    }
}

fn execute_command(state: &mut EditorState, command: &str) -> KeyAction {
    match command {
        "w" => {
            match save_file(state) {
                Ok(message) => state.message = message,
                Err(error) => state.message = error,
            }
            state.mode = EditorMode::Normal;
            KeyAction::Continue
        }
        "q" => {
            if state.dirty {
                state.mode = EditorMode::Normal;
                state.message = "No write since last change (add ! to override)".to_string();
                KeyAction::Continue
            } else {
                KeyAction::Exit(None)
            }
        }
        "q!" => KeyAction::Exit(Some("nvim: discarded changes".to_string())),
        "wq" | "x" => match save_file(state) {
            Ok(message) => KeyAction::Exit(Some(message)),
            Err(error) => {
                state.mode = EditorMode::Normal;
                state.message = error;
                KeyAction::Continue
            }
        },
        _ => {
            state.mode = EditorMode::Normal;
            state.message = format!("Not an editor command: {}", command);
            KeyAction::Continue
        }
    }
}

fn save_file(state: &mut EditorState) -> Result<String, String> {
    let content = state.lines.join("\n");
    let user = CURRENT_USER.as_str();

    let mut filesystem = FILESYSTEM.lock().unwrap();
    let parent_path = &state.path[..state.path.len() - 1];
    let file_name = &state.path[state.path.len() - 1];

    let parent = match get_node_mut(&mut filesystem, parent_path) {
        Some(Node::Directory { children, .. }) => children,
        Some(_) => return Err("nvim: parent is not a directory".to_string()),
        None => return Err("nvim: parent directory does not exist".to_string()),
    };

    if let Some(existing) = parent.get_mut(file_name) {
        match existing {
            Node::File {
                content: existing_content,
                owner,
                protected,
                ..
            } => {
                if *protected {
                    return Err("nvim: file is protected".to_string());
                }

                if owner.as_str() != user && user != "root" {
                    return Err("nvim: permission denied".to_string());
                }

                *existing_content = content;
            }
            Node::Directory { .. } => return Err("nvim: target is a directory".to_string()),
            Node::Symlink { .. } => return Err("nvim: cannot write symlink target".to_string()),
        }
    } else {
        parent.insert(
            file_name.clone(),
            Node::File {
                content,
                permissions: 0o644,
                owner: user.to_string(),
                protected: false,
            },
        );
    }

    state.dirty = false;
    Ok(format!("wrote {}", state.display_path))
}

fn char_len(text: &str) -> usize {
    text.chars().count()
}

fn byte_index(text: &str, char_index: usize) -> usize {
    if char_index == 0 {
        return 0;
    }

    text.char_indices()
        .nth(char_index)
        .map(|(index, _)| index)
        .unwrap_or(text.len())
}

fn insert_char(state: &mut EditorState, ch: char) {
    let line = &mut state.lines[state.cursor_row];
    let index = byte_index(line, state.cursor_col);
    line.insert(index, ch);
    state.cursor_col += 1;
    state.dirty = true;
}

fn backspace(state: &mut EditorState) {
    if state.cursor_col > 0 {
        let line = &mut state.lines[state.cursor_row];
        let end = byte_index(line, state.cursor_col);
        let start = byte_index(line, state.cursor_col - 1);
        line.replace_range(start..end, "");
        state.cursor_col -= 1;
        state.dirty = true;
        return;
    }

    if state.cursor_row == 0 {
        return;
    }

    let current = state.lines.remove(state.cursor_row);
    state.cursor_row -= 1;
    let prev_len = char_len(&state.lines[state.cursor_row]);
    state.lines[state.cursor_row].push_str(&current);
    state.cursor_col = prev_len;
    state.dirty = true;
}

fn split_line(state: &mut EditorState) {
    let current = &mut state.lines[state.cursor_row];
    let split_at = byte_index(current, state.cursor_col);
    let trailing = current.split_off(split_at);
    state.cursor_row += 1;
    state.cursor_col = 0;
    state.lines.insert(state.cursor_row, trailing);
    state.dirty = true;
}

fn delete_under_cursor(state: &mut EditorState) {
    let row = state.cursor_row;
    let col = state.cursor_col;
    let len = char_len(&state.lines[row]);

    if col < len {
        let line = &mut state.lines[row];
        let start = byte_index(line, col);
        let end = byte_index(line, col + 1);
        line.replace_range(start..end, "");
        state.dirty = true;
        return;
    }

    if row + 1 < state.lines.len() {
        let next = state.lines.remove(row + 1);
        state.lines[row].push_str(&next);
        state.dirty = true;
    }
}

fn move_left(state: &mut EditorState) {
    if state.cursor_col > 0 {
        state.cursor_col -= 1;
    } else if state.cursor_row > 0 {
        state.cursor_row -= 1;
        state.cursor_col = char_len(&state.lines[state.cursor_row]);
    }
}

fn move_right(state: &mut EditorState) {
    let len = char_len(&state.lines[state.cursor_row]);
    if state.cursor_col < len {
        state.cursor_col += 1;
    } else if state.cursor_row + 1 < state.lines.len() {
        state.cursor_row += 1;
        state.cursor_col = 0;
    }
}

fn move_up(state: &mut EditorState) {
    if state.cursor_row > 0 {
        state.cursor_row -= 1;
    }
    state.cursor_col = state
        .cursor_col
        .min(char_len(&state.lines[state.cursor_row]));
}

fn move_down(state: &mut EditorState) {
    if state.cursor_row + 1 < state.lines.len() {
        state.cursor_row += 1;
    }
    state.cursor_col = state
        .cursor_col
        .min(char_len(&state.lines[state.cursor_row]));
}

fn line_with_cursor(line: &str, cursor_col: usize) -> String {
    let cursor_col = cursor_col.min(char_len(line));
    let marker = "\x1b[1;33m|\x1b[0m";
    let split_at = byte_index(line, cursor_col);
    format!("{}{}{}", &line[..split_at], marker, &line[split_at..])
}

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

#[derive(Clone, Copy, PartialEq, Eq)]
enum PendingOp {
    None,
    Delete,
    Change,
    Yank,
}

#[derive(Clone)]
struct UndoEntry {
    lines: Vec<String>,
    cursor_row: usize,
    cursor_col: usize,
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
    pending_op: PendingOp,
    count: Option<usize>,
    register: String,
    undo_stack: Vec<UndoEntry>,
    redo_stack: Vec<UndoEntry>,
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
            pending_op: PendingOp::None,
            count: None,
            register: String::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
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
                line_with_cursor(&state.lines[file_row], state.cursor_col, state.mode)
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
    let key = event.key();
    let key_str = key.as_str();

    // Handle Ctrl+R for redo
    if event.ctrl_key() && key_str.eq_ignore_ascii_case("r") {
        if redo(state) {
            state.message = "redone".to_string();
        } else {
            state.message = "already at newest change".to_string();
        }
        return KeyAction::Continue;
    }

    // Handle Escape to cancel pending operation
    if key_str == "Escape" {
        state.pending_op = PendingOp::None;
        state.count = None;
        state.message = "i: insert | :w write | :q quit | :wq write+quit".to_string();
        return KeyAction::Continue;
    }

    // Accumulate count prefix (1-9 start, 0-9 continue)
    if let Some(digit) = key_str.chars().next().filter(|c| c.is_ascii_digit()) {
        if digit != '0' || state.count.is_some() {
            let current = state.count.unwrap_or(0);
            state.count = Some(current * 10 + digit.to_digit(10).unwrap() as usize);
            return KeyAction::Continue;
        }
    }

    let count = state.count.take().unwrap_or(1);

    // Handle operator-pending mode (d, c, y followed by motion)
    if state.pending_op != PendingOp::None {
        let op = state.pending_op;
        state.pending_op = PendingOp::None;

        match key_str {
            // dd, cc, yy - operate on whole line(s)
            "d" if op == PendingOp::Delete => delete_lines(state, count),
            "c" if op == PendingOp::Change => {
                delete_lines(state, count);
                state.mode = EditorMode::Insert;
                state.message = "-- INSERT -- (Ctrl+Esc to exit)".to_string();
            }
            "y" if op == PendingOp::Yank => yank_lines(state, count),

            // Motions with operators
            "w" => operate_word_forward(state, op, count),
            "e" => operate_word_end(state, op, count),
            "b" => operate_word_backward(state, op, count),
            "0" | "Home" => operate_to_col(state, op, 0),
            "$" | "End" => operate_to_col(state, op, char_len(&state.lines[state.cursor_row])),
            "g" => {
                // dgg, cgg, ygg - operate to start of file
                operate_to_line(state, op, 0);
            }
            "G" => {
                // dG, cG, yG - operate to end of file
                operate_to_line(state, op, state.lines.len().saturating_sub(1));
            }
            "i" => {
                // diw, daw, etc. - inner/around word (simplified: just word)
                // We'll handle this as word under cursor
            }
            _ => {
                state.message = "Unknown motion".to_string();
            }
        }

        if op == PendingOp::Change && state.mode == EditorMode::Normal {
            state.mode = EditorMode::Insert;
            state.message = "-- INSERT -- (Ctrl+Esc to exit)".to_string();
        }

        return KeyAction::Continue;
    }

    // Normal mode commands
    match key_str {
        // Basic movement
        "h" | "ArrowLeft" => {
            for _ in 0..count {
                move_left(state);
            }
        }
        "j" | "ArrowDown" => {
            for _ in 0..count {
                move_down(state);
            }
        }
        "k" | "ArrowUp" => {
            for _ in 0..count {
                move_up(state);
            }
        }
        "l" | "ArrowRight" => {
            for _ in 0..count {
                move_right(state);
            }
        }

        // Line position
        "0" | "Home" => state.cursor_col = 0,
        "^" => move_to_first_non_blank(state),
        "$" | "End" => {
            state.cursor_col = char_len(&state.lines[state.cursor_row])
                .saturating_sub(1)
                .max(0)
        }

        // Word motions
        "w" => {
            for _ in 0..count {
                move_word_forward(state);
            }
        }
        "W" => {
            for _ in 0..count {
                move_word_forward_big(state);
            }
        }
        "e" => {
            for _ in 0..count {
                move_word_end(state);
            }
        }
        "E" => {
            for _ in 0..count {
                move_word_end_big(state);
            }
        }
        "b" => {
            for _ in 0..count {
                move_word_backward(state);
            }
        }
        "B" => {
            for _ in 0..count {
                move_word_backward_big(state);
            }
        }

        // Line navigation
        "g" => {
            // gg - go to first line (or line N with count)
            state.cursor_row = count
                .saturating_sub(1)
                .min(state.lines.len().saturating_sub(1));
            state.cursor_col = state
                .cursor_col
                .min(char_len(&state.lines[state.cursor_row]));
        }
        "G" => {
            // G - go to last line (or line N with count)
            if count > 1 {
                state.cursor_row = (count - 1).min(state.lines.len().saturating_sub(1));
            } else {
                state.cursor_row = state.lines.len().saturating_sub(1);
            }
            state.cursor_col = state
                .cursor_col
                .min(char_len(&state.lines[state.cursor_row]));
        }

        // Insert modes
        "i" => {
            save_undo(state);
            state.mode = EditorMode::Insert;
            state.message = "-- INSERT -- (Ctrl+Esc to exit)".to_string();
        }
        "I" => {
            save_undo(state);
            move_to_first_non_blank(state);
            state.mode = EditorMode::Insert;
            state.message = "-- INSERT -- (Ctrl+Esc to exit)".to_string();
        }
        "a" => {
            save_undo(state);
            move_right(state);
            state.mode = EditorMode::Insert;
            state.message = "-- INSERT -- (Ctrl+Esc to exit)".to_string();
        }
        "A" => {
            save_undo(state);
            state.cursor_col = char_len(&state.lines[state.cursor_row]);
            state.mode = EditorMode::Insert;
            state.message = "-- INSERT -- (Ctrl+Esc to exit)".to_string();
        }
        "o" => {
            save_undo(state);
            let insert_at = state.cursor_row + 1;
            state.lines.insert(insert_at, String::new());
            state.cursor_row = insert_at;
            state.cursor_col = 0;
            state.mode = EditorMode::Insert;
            state.dirty = true;
            state.message = "-- INSERT -- (Ctrl+Esc to exit)".to_string();
        }
        "O" => {
            save_undo(state);
            state.lines.insert(state.cursor_row, String::new());
            state.cursor_col = 0;
            state.mode = EditorMode::Insert;
            state.dirty = true;
            state.message = "-- INSERT -- (Ctrl+Esc to exit)".to_string();
        }

        // Delete/change/yank operators
        "d" => {
            state.pending_op = PendingOp::Delete;
            state.count = Some(count);
        }
        "c" => {
            state.pending_op = PendingOp::Change;
            state.count = Some(count);
        }
        "y" => {
            state.pending_op = PendingOp::Yank;
            state.count = Some(count);
        }

        // Single char operations
        "x" => {
            save_undo(state);
            for _ in 0..count {
                delete_under_cursor(state);
            }
        }
        "X" => {
            save_undo(state);
            for _ in 0..count {
                if state.cursor_col > 0 {
                    state.cursor_col -= 1;
                    delete_under_cursor(state);
                }
            }
        }
        "r" => {
            // r is handled specially - need next char
            state.message = "r: replace char".to_string();
            state.pending_op = PendingOp::None;
            // We'll use count field to signal replace mode
            state.count = Some(count);
            state.register = "r".to_string();
        }
        "s" => {
            // s - substitute char (delete and insert)
            save_undo(state);
            for _ in 0..count {
                delete_under_cursor(state);
            }
            state.mode = EditorMode::Insert;
            state.message = "-- INSERT -- (Ctrl+Esc to exit)".to_string();
        }
        "S" | "C" => {
            // S/C - change whole line / change to end of line
            save_undo(state);
            if key_str == "S" {
                state.lines[state.cursor_row].clear();
                state.cursor_col = 0;
            } else {
                let line = &mut state.lines[state.cursor_row];
                let pos = byte_index(line, state.cursor_col);
                line.truncate(pos);
            }
            state.dirty = true;
            state.mode = EditorMode::Insert;
            state.message = "-- INSERT -- (Ctrl+Esc to exit)".to_string();
        }
        "D" => {
            // D - delete to end of line
            save_undo(state);
            let line = &mut state.lines[state.cursor_row];
            let pos = byte_index(line, state.cursor_col);
            state.register = line[pos..].to_string();
            line.truncate(pos);
            state.dirty = true;
            state.cursor_col = state
                .cursor_col
                .min(char_len(&state.lines[state.cursor_row]).saturating_sub(1));
        }

        // Paste
        "p" => {
            // Paste after cursor
            if !state.register.is_empty() {
                save_undo(state);
                for _ in 0..count {
                    paste_after(state);
                }
            }
        }
        "P" => {
            // Paste before cursor
            if !state.register.is_empty() {
                save_undo(state);
                for _ in 0..count {
                    paste_before(state);
                }
            }
        }

        // Undo/Redo
        "u" => {
            if undo(state) {
                state.message = "undone".to_string();
            } else {
                state.message = "already at oldest change".to_string();
            }
        }

        // Ctrl+R for redo is handled separately since we need ctrl check

        // Join lines
        "J" => {
            save_undo(state);
            for _ in 0..count {
                join_line(state);
            }
        }

        // Command mode
        ":" => {
            state.mode = EditorMode::Command;
            state.command_line.clear();
        }

        _ => {
            // Check if this is a replacement char after 'r'
            if state.register == "r" && key_str.chars().count() == 1 {
                if let Some(ch) = key_str.chars().next() {
                    save_undo(state);
                    let repeat = state.count.unwrap_or(1);
                    for _ in 0..repeat {
                        replace_char(state, ch);
                        if state.cursor_col + 1 < char_len(&state.lines[state.cursor_row]) {
                            state.cursor_col += 1;
                        }
                    }
                    state.cursor_col = state.cursor_col.saturating_sub(1);
                }
                state.register.clear();
                state.count = None;
                state.message = "i: insert | :w write | :q quit | :wq write+quit".to_string();
            }
        }
    }

    KeyAction::Continue
}

fn handle_insert_mode(state: &mut EditorState, event: &KeyboardEvent) -> KeyAction {
    // Handle Escape: Escape key, Ctrl+Escape, or Ctrl+[ (standard vim escape)
    let is_escape =
        event.key().as_str() == "Escape" || (event.ctrl_key() && event.key().as_str() == "[");

    if is_escape {
        state.mode = EditorMode::Normal;
        state.message = "i: insert | :w write | :q quit | :wq write+quit".to_string();
        return KeyAction::Continue;
    }

    match event.key().as_str() {
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
    // Handle Escape: Escape key, Ctrl+Escape, or Ctrl+[ (standard vim escape)
    let is_escape =
        event.key().as_str() == "Escape" || (event.ctrl_key() && event.key().as_str() == "[");

    if is_escape {
        state.mode = EditorMode::Normal;
        state.command_line.clear();
        state.message = "i: insert | :w write | :q quit | :wq write+quit".to_string();
        return KeyAction::Continue;
    }

    match event.key().as_str() {
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

// Word classification
fn is_word_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

fn is_whitespace(ch: char) -> bool {
    ch.is_whitespace()
}

fn is_big_word_char(ch: char) -> bool {
    !ch.is_whitespace()
}

// Move to first non-blank character
fn move_to_first_non_blank(state: &mut EditorState) {
    let line = &state.lines[state.cursor_row];
    state.cursor_col = line.chars().position(|c| !c.is_whitespace()).unwrap_or(0);
}

// Word forward (w)
fn move_word_forward(state: &mut EditorState) {
    let line = &state.lines[state.cursor_row];
    let chars: Vec<char> = line.chars().collect();
    let len = chars.len();

    if state.cursor_col >= len {
        // Move to next line
        if state.cursor_row + 1 < state.lines.len() {
            state.cursor_row += 1;
            state.cursor_col = 0;
            move_to_first_non_blank(state);
        }
        return;
    }

    let mut col = state.cursor_col;

    // Skip current word
    if col < len && is_word_char(chars[col]) {
        while col < len && is_word_char(chars[col]) {
            col += 1;
        }
    } else if col < len && !is_whitespace(chars[col]) {
        while col < len && !is_word_char(chars[col]) && !is_whitespace(chars[col]) {
            col += 1;
        }
    }

    // Skip whitespace
    while col < len && is_whitespace(chars[col]) {
        col += 1;
    }

    if col >= len && state.cursor_row + 1 < state.lines.len() {
        state.cursor_row += 1;
        state.cursor_col = 0;
        move_to_first_non_blank(state);
    } else {
        state.cursor_col = col.min(len.saturating_sub(1));
    }
}

// Word forward big (W)
fn move_word_forward_big(state: &mut EditorState) {
    let line = &state.lines[state.cursor_row];
    let chars: Vec<char> = line.chars().collect();
    let len = chars.len();

    if state.cursor_col >= len {
        if state.cursor_row + 1 < state.lines.len() {
            state.cursor_row += 1;
            state.cursor_col = 0;
            move_to_first_non_blank(state);
        }
        return;
    }

    let mut col = state.cursor_col;

    // Skip non-whitespace
    while col < len && is_big_word_char(chars[col]) {
        col += 1;
    }

    // Skip whitespace
    while col < len && is_whitespace(chars[col]) {
        col += 1;
    }

    if col >= len && state.cursor_row + 1 < state.lines.len() {
        state.cursor_row += 1;
        state.cursor_col = 0;
        move_to_first_non_blank(state);
    } else {
        state.cursor_col = col.min(len.saturating_sub(1));
    }
}

// Word end (e)
fn move_word_end(state: &mut EditorState) {
    let line = &state.lines[state.cursor_row];
    let chars: Vec<char> = line.chars().collect();
    let len = chars.len();

    if len == 0 || state.cursor_col >= len.saturating_sub(1) {
        if state.cursor_row + 1 < state.lines.len() {
            state.cursor_row += 1;
            state.cursor_col = 0;
            move_word_end(state);
        }
        return;
    }

    let mut col = state.cursor_col + 1;

    // Skip whitespace
    while col < len && is_whitespace(chars[col]) {
        col += 1;
    }

    // Move to end of word
    if col < len && is_word_char(chars[col]) {
        while col + 1 < len && is_word_char(chars[col + 1]) {
            col += 1;
        }
    } else if col < len {
        while col + 1 < len && !is_word_char(chars[col + 1]) && !is_whitespace(chars[col + 1]) {
            col += 1;
        }
    }

    state.cursor_col = col.min(len.saturating_sub(1));
}

// Word end big (E)
fn move_word_end_big(state: &mut EditorState) {
    let line = &state.lines[state.cursor_row];
    let chars: Vec<char> = line.chars().collect();
    let len = chars.len();

    if len == 0 || state.cursor_col >= len.saturating_sub(1) {
        if state.cursor_row + 1 < state.lines.len() {
            state.cursor_row += 1;
            state.cursor_col = 0;
            move_word_end_big(state);
        }
        return;
    }

    let mut col = state.cursor_col + 1;

    // Skip whitespace
    while col < len && is_whitespace(chars[col]) {
        col += 1;
    }

    // Move to end of WORD
    while col + 1 < len && is_big_word_char(chars[col + 1]) {
        col += 1;
    }

    state.cursor_col = col.min(len.saturating_sub(1));
}

// Word backward (b)
fn move_word_backward(state: &mut EditorState) {
    if state.cursor_col == 0 {
        if state.cursor_row > 0 {
            state.cursor_row -= 1;
            state.cursor_col = char_len(&state.lines[state.cursor_row]);
            if state.cursor_col > 0 {
                state.cursor_col -= 1;
                move_word_backward(state);
            }
        }
        return;
    }

    let line = &state.lines[state.cursor_row];
    let chars: Vec<char> = line.chars().collect();
    let mut col = state.cursor_col.saturating_sub(1);

    // Skip whitespace
    while col > 0 && is_whitespace(chars[col]) {
        col -= 1;
    }

    // Move to start of word
    if is_word_char(chars[col]) {
        while col > 0 && is_word_char(chars[col - 1]) {
            col -= 1;
        }
    } else if !is_whitespace(chars[col]) {
        while col > 0 && !is_word_char(chars[col - 1]) && !is_whitespace(chars[col - 1]) {
            col -= 1;
        }
    }

    state.cursor_col = col;
}

// Word backward big (B)
fn move_word_backward_big(state: &mut EditorState) {
    if state.cursor_col == 0 {
        if state.cursor_row > 0 {
            state.cursor_row -= 1;
            state.cursor_col = char_len(&state.lines[state.cursor_row]);
            if state.cursor_col > 0 {
                state.cursor_col -= 1;
                move_word_backward_big(state);
            }
        }
        return;
    }

    let line = &state.lines[state.cursor_row];
    let chars: Vec<char> = line.chars().collect();
    let mut col = state.cursor_col.saturating_sub(1);

    // Skip whitespace
    while col > 0 && is_whitespace(chars[col]) {
        col -= 1;
    }

    // Move to start of WORD
    while col > 0 && is_big_word_char(chars[col - 1]) {
        col -= 1;
    }

    state.cursor_col = col;
}

// Delete lines (dd)
fn delete_lines(state: &mut EditorState, count: usize) {
    save_undo(state);
    let end_row = (state.cursor_row + count).min(state.lines.len());
    let deleted: Vec<String> = state.lines.drain(state.cursor_row..end_row).collect();
    state.register = deleted.join("\n");
    state.dirty = true;

    if state.lines.is_empty() {
        state.lines.push(String::new());
    }

    state.cursor_row = state.cursor_row.min(state.lines.len().saturating_sub(1));
    state.cursor_col = state
        .cursor_col
        .min(char_len(&state.lines[state.cursor_row]));
}

// Yank lines (yy)
fn yank_lines(state: &mut EditorState, count: usize) {
    let end_row = (state.cursor_row + count).min(state.lines.len());
    let yanked: Vec<&String> = state.lines[state.cursor_row..end_row].iter().collect();
    state.register = yanked
        .iter()
        .map(|s| s.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    state.message = format!("{} line(s) yanked", end_row - state.cursor_row);
}

// Save state for undo before making changes
fn save_undo(state: &mut EditorState) {
    state.undo_stack.push(UndoEntry {
        lines: state.lines.clone(),
        cursor_row: state.cursor_row,
        cursor_col: state.cursor_col,
    });
    state.redo_stack.clear();
    // Limit undo history
    if state.undo_stack.len() > 100 {
        state.undo_stack.remove(0);
    }
}

// Undo last change
fn undo(state: &mut EditorState) -> bool {
    if let Some(entry) = state.undo_stack.pop() {
        // Save current state for redo
        state.redo_stack.push(UndoEntry {
            lines: state.lines.clone(),
            cursor_row: state.cursor_row,
            cursor_col: state.cursor_col,
        });
        state.lines = entry.lines;
        state.cursor_row = entry.cursor_row;
        state.cursor_col = entry.cursor_col;
        state.dirty = true;
        true
    } else {
        false
    }
}

// Redo last undone change
fn redo(state: &mut EditorState) -> bool {
    if let Some(entry) = state.redo_stack.pop() {
        state.undo_stack.push(UndoEntry {
            lines: state.lines.clone(),
            cursor_row: state.cursor_row,
            cursor_col: state.cursor_col,
        });
        state.lines = entry.lines;
        state.cursor_row = entry.cursor_row;
        state.cursor_col = entry.cursor_col;
        state.dirty = true;
        true
    } else {
        false
    }
}

// Operate with motion (dw, cw, yw, etc.)
fn operate_word_forward(state: &mut EditorState, op: PendingOp, count: usize) {
    let start_row = state.cursor_row;
    let start_col = state.cursor_col;

    // Calculate end position without clamping
    let mut end_row = start_row;
    let mut end_col = start_col;

    for _ in 0..count {
        let line = &state.lines[end_row];
        let chars: Vec<char> = line.chars().collect();
        let len = chars.len();

        if end_col >= len {
            // Move to next line
            if end_row + 1 < state.lines.len() {
                end_row += 1;
                end_col = 0;
                // Skip leading whitespace
                let next_line: Vec<char> = state.lines[end_row].chars().collect();
                while end_col < next_line.len() && next_line[end_col].is_whitespace() {
                    end_col += 1;
                }
            }
            continue;
        }

        let mut col = end_col;

        // Skip current word
        if col < len && is_word_char(chars[col]) {
            while col < len && is_word_char(chars[col]) {
                col += 1;
            }
        } else if col < len && !is_whitespace(chars[col]) {
            while col < len && !is_word_char(chars[col]) && !is_whitespace(chars[col]) {
                col += 1;
            }
        }

        // Skip whitespace
        while col < len && is_whitespace(chars[col]) {
            col += 1;
        }

        if col >= len && end_row + 1 < state.lines.len() {
            end_row += 1;
            end_col = 0;
            // Skip leading whitespace on new line
            let next_line: Vec<char> = state.lines[end_row].chars().collect();
            while end_col < next_line.len() && next_line[end_col].is_whitespace() {
                end_col += 1;
            }
        } else {
            end_col = col; // Don't clamp - keep actual position
        }
    }

    // Handle same line deletion
    if end_row == start_row {
        let line_len = char_len(&state.lines[start_row]);
        // Use the calculated end_col, but cap at line length
        let end_col = end_col.min(line_len);

        if end_col > start_col {
            let line = &state.lines[start_row];
            let start_byte = byte_index(line, start_col);
            let end_byte = byte_index(line, end_col);
            let deleted = line[start_byte..end_byte].to_string();

            match op {
                PendingOp::Delete | PendingOp::Change => {
                    save_undo(state);
                    state.lines[start_row].replace_range(start_byte..end_byte, "");
                    state.cursor_col = start_col;
                    state.dirty = true;
                }
                PendingOp::Yank => {
                    state.cursor_row = start_row;
                    state.cursor_col = start_col;
                }
                PendingOp::None => {}
            }
            state.register = deleted;
        }
    } else {
        // Multi-line deletion: delete from start_col to end of start_row, then full lines, then to cursor on final row
        match op {
            PendingOp::Delete | PendingOp::Change => {
                save_undo(state);
                // Delete from cursor to end of starting line
                let line = &state.lines[start_row];
                let start_byte = byte_index(line, start_col);
                let deleted_first = line[start_byte..].to_string();
                state.lines[start_row].truncate(start_byte);

                // Join with content after cursor on final line
                if end_row < state.lines.len() {
                    let final_line = &state.lines[end_row];
                    let end_byte = byte_index(final_line, end_col.min(char_len(final_line)));
                    let remaining = final_line[end_byte..].to_string();

                    // Remove lines between start and end
                    for _ in start_row + 1..=end_row {
                        if start_row + 1 < state.lines.len() {
                            state.lines.remove(start_row + 1);
                        }
                    }

                    state.lines[start_row].push_str(&remaining);
                }

                state.cursor_row = start_row;
                state.cursor_col = start_col;
                state.register = deleted_first;
                state.dirty = true;
            }
            PendingOp::Yank => {
                state.cursor_row = start_row;
                state.cursor_col = start_col;
            }
            PendingOp::None => {}
        }
    }
}

fn operate_word_end(state: &mut EditorState, op: PendingOp, count: usize) {
    let start_row = state.cursor_row;
    let start_col = state.cursor_col;

    for _ in 0..count {
        move_word_end(state);
    }

    if state.cursor_row == start_row {
        let end_col = (state.cursor_col + 1).min(char_len(&state.lines[start_row]));
        let line = &state.lines[start_row];
        let start_byte = byte_index(line, start_col);
        let end_byte = byte_index(line, end_col);
        let deleted = line[start_byte..end_byte].to_string();

        match op {
            PendingOp::Delete | PendingOp::Change => {
                save_undo(state);
                state.lines[start_row].replace_range(start_byte..end_byte, "");
                state.cursor_col = start_col;
                state.dirty = true;
            }
            PendingOp::Yank => {
                state.cursor_row = start_row;
                state.cursor_col = start_col;
            }
            PendingOp::None => {}
        }
        state.register = deleted;
    }
}

fn operate_word_backward(state: &mut EditorState, op: PendingOp, count: usize) {
    let start_row = state.cursor_row;
    let start_col = state.cursor_col;

    for _ in 0..count {
        move_word_backward(state);
    }

    if state.cursor_row == start_row && state.cursor_col < start_col {
        let line = &state.lines[start_row];
        let start_byte = byte_index(line, state.cursor_col);
        let end_byte = byte_index(line, start_col);
        let deleted = line[start_byte..end_byte].to_string();

        match op {
            PendingOp::Delete | PendingOp::Change => {
                save_undo(state);
                state.lines[start_row].replace_range(start_byte..end_byte, "");
                state.dirty = true;
            }
            PendingOp::Yank => {
                state.cursor_row = start_row;
                state.cursor_col = start_col;
            }
            PendingOp::None => {}
        }
        state.register = deleted;
    }
}

fn operate_to_col(state: &mut EditorState, op: PendingOp, target_col: usize) {
    let row = state.cursor_row;
    let start_col = state.cursor_col.min(target_col);
    let end_col = state.cursor_col.max(target_col);

    let line = &state.lines[row];
    let start_byte = byte_index(line, start_col);
    let end_byte = byte_index(line, end_col);
    let deleted = line[start_byte..end_byte].to_string();

    match op {
        PendingOp::Delete | PendingOp::Change => {
            save_undo(state);
            state.lines[row].replace_range(start_byte..end_byte, "");
            state.cursor_col = start_col;
            state.dirty = true;
        }
        PendingOp::Yank => {}
        PendingOp::None => {}
    }
    state.register = deleted;
}

fn operate_to_line(state: &mut EditorState, op: PendingOp, target_row: usize) {
    let start_row = state.cursor_row.min(target_row);
    let end_row = state.cursor_row.max(target_row) + 1;
    let end_row = end_row.min(state.lines.len());

    match op {
        PendingOp::Delete | PendingOp::Change => {
            save_undo(state);
            let deleted: Vec<String> = state.lines.drain(start_row..end_row).collect();
            state.register = deleted.join("\n");
            state.dirty = true;

            if state.lines.is_empty() {
                state.lines.push(String::new());
            }

            state.cursor_row = start_row.min(state.lines.len().saturating_sub(1));
            state.cursor_col = 0;
        }
        PendingOp::Yank => {
            let yanked: Vec<&String> = state.lines[start_row..end_row].iter().collect();
            state.register = yanked
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join("\n");
            state.message = format!("{} line(s) yanked", end_row - start_row);
        }
        PendingOp::None => {}
    }
}

// Replace character (r)
fn replace_char(state: &mut EditorState, ch: char) {
    let row = state.cursor_row;
    let col = state.cursor_col;
    let len = char_len(&state.lines[row]);

    if col < len {
        let line = &mut state.lines[row];
        let start = byte_index(line, col);
        let end = byte_index(line, col + 1);
        line.replace_range(start..end, &ch.to_string());
        state.dirty = true;
    }
}

// Paste after cursor
fn paste_after(state: &mut EditorState) {
    if state.register.is_empty() {
        return;
    }

    // Check if register contains newlines (line-wise paste)
    if state.register.contains('\n') {
        let lines: Vec<&str> = state.register.split('\n').collect();
        for (i, line) in lines.iter().enumerate() {
            state
                .lines
                .insert(state.cursor_row + 1 + i, line.to_string());
        }
        state.cursor_row += 1;
        state.cursor_col = 0;
    } else {
        // Character-wise paste
        let line = &mut state.lines[state.cursor_row];
        let insert_at = byte_index(line, state.cursor_col + 1);
        line.insert_str(insert_at, &state.register);
        state.cursor_col += 1;
    }
    state.dirty = true;
}

// Paste before cursor
fn paste_before(state: &mut EditorState) {
    if state.register.is_empty() {
        return;
    }

    if state.register.contains('\n') {
        let lines: Vec<&str> = state.register.split('\n').collect();
        for (i, line) in lines.iter().enumerate() {
            state.lines.insert(state.cursor_row + i, line.to_string());
        }
        state.cursor_col = 0;
    } else {
        let line = &mut state.lines[state.cursor_row];
        let insert_at = byte_index(line, state.cursor_col);
        line.insert_str(insert_at, &state.register);
    }
    state.dirty = true;
}

// Join lines (J)
fn join_line(state: &mut EditorState) {
    if state.cursor_row + 1 >= state.lines.len() {
        return;
    }

    let current_len = char_len(&state.lines[state.cursor_row]);
    let next = state.lines.remove(state.cursor_row + 1);
    let trimmed = next.trim_start();

    if !state.lines[state.cursor_row].is_empty() && !trimmed.is_empty() {
        state.lines[state.cursor_row].push(' ');
        state.cursor_col = current_len;
    }
    state.lines[state.cursor_row].push_str(trimmed);
    state.dirty = true;
}

fn line_with_cursor(line: &str, cursor_col: usize, mode: EditorMode) -> String {
    let len = char_len(line);
    let cursor_col = cursor_col.min(len);

    match mode {
        EditorMode::Insert => {
            // Insert mode: line cursor (|) before the character
            let marker = "\x1b[1;33m|\x1b[0m";
            let split_at = byte_index(line, cursor_col);
            format!("{}{}{}", &line[..split_at], marker, &line[split_at..])
        }
        EditorMode::Normal | EditorMode::Command => {
            // Normal/Command mode: block cursor overlaying the character
            // Use code 99 to adjust background offset for nvim cursor
            if cursor_col >= len {
                // Cursor is at end of line, show block cursor as highlighted underscore
                format!("{}\x1b[1;32m_\x1b[0m", line)
            } else {
                // Highlight the character under cursor in bright color
                // Code 99 = nvim cursor offset adjustment
                let before = byte_index(line, cursor_col);
                let after = byte_index(line, cursor_col + 1);
                let char_under = &line[before..after];
                format!(
                    "{}\x1b[99;1;30;42m{}\x1b[0m{}",
                    &line[..before],
                    char_under,
                    &line[after..]
                )
            }
        }
    }
}

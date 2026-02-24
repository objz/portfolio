use std::cell::RefCell;
use std::collections::VecDeque;

#[derive(Debug, Clone, PartialEq)]
pub enum LineType {
    Normal,
    Command,
    Output,
    Boot,
    Typing,
}

#[derive(Debug, Clone)]
pub struct BufferLine {
    pub content: String,
    pub line_type: LineType,
    pub color: Option<String>,
    pub wrapped_lines: Vec<String>,
}

impl BufferLine {
    pub fn new(content: String, line_type: LineType, color: Option<String>) -> Self {
        Self {
            content,
            line_type,
            color,
            wrapped_lines: Vec::new(),
        }
    }

    pub fn calc_wrapping(&mut self, max_width: usize) {
        self.wrapped_lines.clear();

        if self.content.contains('\u{1b}') {
            self.wrapped_lines.push(self.content.clone());
            return;
        }

        let chars: Vec<char> = self.content.chars().collect();

        if chars.len() <= max_width {
            self.wrapped_lines.push(self.content.clone());
            return;
        }

        let mut start = 0;
        while start < chars.len() {
            let end = (start + max_width).min(chars.len());

            if end >= chars.len() {
                let chunk: String = chars[start..].iter().collect();
                self.wrapped_lines.push(chunk);
                break;
            }

            let mut break_point = end;
            for i in (start..end).rev() {
                if chars[i] == ' ' {
                    break_point = i;
                    break;
                }
            }

            let chunk: String = chars[start..break_point].iter().collect();
            self.wrapped_lines.push(chunk);

            start = if break_point < end && chars[break_point] == ' ' {
                break_point + 1
            } else {
                break_point
            };
        }
    }

    pub fn get_line_count(&self) -> usize {
        if self.wrapped_lines.is_empty() {
            1
        } else {
            self.wrapped_lines.len()
        }
    }
}

#[derive(Debug, Clone)]
pub struct TerminalState {
    pub current_input: String,
    pub autosuggestion: String,
    pub cursor_position: usize,
    pub prompt: String,
    pub input_mode: InputMode,
    pub scroll_offset: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InputMode {
    Normal,
    Disabled,
}

impl Default for TerminalState {
    fn default() -> Self {
        Self {
            current_input: String::new(),
            autosuggestion: String::new(),
            cursor_position: 0,
            prompt: "objz@portfolio:~$ ".to_string(),
            input_mode: InputMode::Normal,
            scroll_offset: 0,
        }
    }
}

pub struct LineBuffer {
    buffer: RefCell<VecDeque<BufferLine>>,
    state: RefCell<TerminalState>,
    max_lines: RefCell<usize>,
    terminal_width: RefCell<usize>,
    terminal_height: RefCell<usize>,
}

impl LineBuffer {
    pub fn new() -> Self {
        Self {
            buffer: RefCell::new(VecDeque::new()),
            state: RefCell::new(TerminalState::default()),
            max_lines: RefCell::new(1000), // keep history
            terminal_width: RefCell::new(80),
            terminal_height: RefCell::new(25),
        }
    }

    pub fn set_dimensions(&self, width: usize, height: usize) {
        *self.terminal_width.borrow_mut() = width;
        *self.terminal_height.borrow_mut() = height;

        {
            let mut buffer = self.buffer.borrow_mut();
            for line in buffer.iter_mut() {
                line.calc_wrapping(width);
            }
        }

        self.clamp_scroll_offset();
    }

    pub fn add_line(&self, content: String, line_type: LineType, color: Option<String>) {
        let mut line = BufferLine::new(content, line_type, color);
        let width = *self.terminal_width.borrow();
        line.calc_wrapping(width);

        let mut buffer = self.buffer.borrow_mut();
        buffer.push_back(line);

        let max = *self.max_lines.borrow();
        while buffer.len() > max {
            buffer.pop_front();
        }

        self.auto_scroll_bottom();
    }

    pub fn add_lines(&self, content: &str, line_type: LineType, color: Option<String>) {
        for line in content.lines() {
            self.add_line(line.to_string(), line_type.clone(), color.clone());
        }
    }

    pub fn add_command(&self, prompt: &str, input: &str) {
        let full_command = format!("{}{}", prompt, input);
        self.add_line(full_command, LineType::Command, Some("cyan".to_string()));
    }

    pub fn clear(&self) {
        self.buffer.borrow_mut().clear();
        self.reset_scroll();
    }

    pub fn get_visible_lines(&self, max_visual_lines: usize) -> Vec<BufferLine> {
        if max_visual_lines == 0 {
            return Vec::new();
        }

        let visual_lines = self.flatten_visual_lines();
        if visual_lines.is_empty() {
            return Vec::new();
        }

        let total = visual_lines.len();
        let scroll_offset = {
            let mut state = self.state.borrow_mut();
            let max_offset = total.saturating_sub(1);
            if state.scroll_offset > max_offset {
                state.scroll_offset = max_offset;
            }
            state.scroll_offset
        };

        let end = total.saturating_sub(scroll_offset);
        let start = end.saturating_sub(max_visual_lines);

        visual_lines[start..end].to_vec()
    }

    pub fn update_input(&self, input: String, cursor_pos: usize) {
        let mut state = self.state.borrow_mut();
        state.current_input = input;
        state.cursor_position = cursor_pos.min(state.current_input.chars().count());
    }

    pub fn update_autosuggestion(&self, suggestion: String) {
        self.state.borrow_mut().autosuggestion = suggestion;
    }

    pub fn set_prompt(&self, prompt: String) {
        self.state.borrow_mut().prompt = prompt;
    }

    fn get_state(&self) -> TerminalState {
        self.state.borrow().clone()
    }

    pub fn set_input_mode(&self, mode: InputMode) {
        self.state.borrow_mut().input_mode = mode;
    }

    pub fn reset_scroll(&self) {
        self.state.borrow_mut().scroll_offset = 0;
    }

    pub fn should_auto_scroll(&self) -> bool {
        let state = self.state.borrow();
        state.scroll_offset == 0
    }

    pub fn auto_scroll_bottom(&self) {
        if self.should_auto_scroll() {
            self.reset_scroll();
        }
    }

    pub fn scroll_up(&self, lines: usize) -> bool {
        if lines == 0 {
            return false;
        }

        let mut state = self.state.borrow_mut();
        let max_scroll_offset = self.max_scroll_offset();

        let new_offset = (state.scroll_offset + lines).min(max_scroll_offset);

        if new_offset != state.scroll_offset {
            state.scroll_offset = new_offset;
            true
        } else {
            false
        }
    }

    pub fn scroll_down(&self, lines: usize) -> bool {
        if lines == 0 {
            return false;
        }

        let mut state = self.state.borrow_mut();
        let new_offset = state.scroll_offset.saturating_sub(lines);

        if new_offset != state.scroll_offset {
            state.scroll_offset = new_offset;
            true
        } else {
            false
        }
    }

    fn flatten_visual_lines(&self) -> Vec<BufferLine> {
        let buffer = self.buffer.borrow();
        let mut visual_lines = Vec::new();

        for line in buffer.iter() {
            if line.wrapped_lines.is_empty() {
                visual_lines.push(BufferLine::new(
                    line.content.clone(),
                    line.line_type.clone(),
                    line.color.clone(),
                ));
                continue;
            }

            for wrapped in &line.wrapped_lines {
                visual_lines.push(BufferLine::new(
                    wrapped.clone(),
                    line.line_type.clone(),
                    line.color.clone(),
                ));
            }
        }

        visual_lines
    }

    fn max_scroll_offset(&self) -> usize {
        let total_visual_lines = self.total_visual_line_count();
        let terminal_height = *self.terminal_height.borrow();

        total_visual_lines.saturating_sub(terminal_height)
    }

    fn total_visual_line_count(&self) -> usize {
        self.buffer
            .borrow()
            .iter()
            .map(|line| line.get_line_count())
            .sum()
    }

    fn clamp_scroll_offset(&self) {
        let max_scroll_offset = self.max_scroll_offset();
        let mut state = self.state.borrow_mut();
        if state.scroll_offset > max_scroll_offset {
            state.scroll_offset = max_scroll_offset;
        }
    }
}

thread_local! {
    pub static LINE_BUFFER: LineBuffer = LineBuffer::new();
}

pub fn add_line(content: String, line_type: LineType, color: Option<String>) {
    LINE_BUFFER.with(|buffer| buffer.add_line(content, line_type, color));
}

pub fn add_command_line(prompt: &str, input: &str) {
    LINE_BUFFER.with(|buffer| buffer.add_command(prompt, input));
}

pub fn add_output_lines(output: &str, color: Option<String>) {
    LINE_BUFFER.with(|buffer| buffer.add_lines(output, LineType::Output, color));
}

pub fn clear_buffer() {
    LINE_BUFFER.with(|buffer| buffer.clear());
}

pub fn set_terminal_dimensions(width: usize, height: usize) {
    LINE_BUFFER.with(|buffer| buffer.set_dimensions(width, height));
}

pub fn get_visible_lines(max_lines: usize) -> Vec<BufferLine> {
    LINE_BUFFER.with(|buffer| buffer.get_visible_lines(max_lines))
}

pub fn update_input_state(input: String, cursor_pos: usize) {
    LINE_BUFFER.with(|buffer| buffer.update_input(input, cursor_pos));
}

pub fn update_autosuggestion(suggestion: String) {
    LINE_BUFFER.with(|buffer| buffer.update_autosuggestion(suggestion));
}

pub fn set_current_prompt(prompt: String) {
    LINE_BUFFER.with(|buffer| buffer.set_prompt(prompt));
}

pub fn get_terminal_state() -> TerminalState {
    LINE_BUFFER.with(|buffer| buffer.get_state())
}

pub fn set_input_mode(mode: InputMode) {
    LINE_BUFFER.with(|buffer| buffer.set_input_mode(mode));
}

pub fn auto_scroll_to_bottom() {
    LINE_BUFFER.with(|buffer| buffer.auto_scroll_bottom());
}

pub fn scroll_up(lines: usize) -> bool {
    LINE_BUFFER.with(|buffer| buffer.scroll_up(lines))
}

pub fn scroll_down(lines: usize) -> bool {
    LINE_BUFFER.with(|buffer| buffer.scroll_down(lines))
}

pub fn reset_scroll() {
    LINE_BUFFER.with(|buffer| buffer.reset_scroll());
}

use std::cell::RefCell;
use std::collections::VecDeque;
#[derive(Debug, Clone, PartialEq)]
pub enum LineType {
    Normal,
    Command,
    Output,
    Boot,
    Typing,
    _Prompt,
    _Error,
    System,
}

#[derive(Debug, Clone)]
pub struct BufferLine {
    pub content: String,
    pub line_type: LineType,
    pub color: Option<String>,
    pub _timestamp: f64,
    pub wrapped_lines: Vec<String>,
}

impl BufferLine {
    pub fn new(content: String, line_type: LineType, color: Option<String>) -> Self {
        Self {
            content,
            line_type,
            color,
            _timestamp: js_sys::Date::now(),
            wrapped_lines: Vec::new(),
        }
    }

    pub fn calc_wrapping(&mut self, max_width: usize) {
        self.wrapped_lines.clear();

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
    pub cursor_position: usize,
    pub prompt: String,
    pub input_mode: InputMode,
    pub scroll_offset: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InputMode {
    Normal,
    Processing,
    Disabled,
}

impl Default for TerminalState {
    fn default() -> Self {
        Self {
            current_input: String::new(),
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
    max_scroll_lines: RefCell<usize>,
}

impl LineBuffer {
    pub fn new() -> Self {
        Self {
            buffer: RefCell::new(VecDeque::new()),
            state: RefCell::new(TerminalState::default()),
            max_lines: RefCell::new(1000), // keep history
            terminal_width: RefCell::new(80),
            terminal_height: RefCell::new(25),
            max_scroll_lines: RefCell::new(100),
        }
    }

    pub fn set_dimensions(&self, width: usize, height: usize) {
        *self.terminal_width.borrow_mut() = width;
        *self.terminal_height.borrow_mut() = height;

        let width_copy = width;
        let mut buffer = self.buffer.borrow_mut();
        for line in buffer.iter_mut() {
            line.calc_wrapping(width_copy);
        }
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

        self.reset_scroll();
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
        let buffer = self.buffer.borrow();
        let state = self.state.borrow();

        let mut visual_lines = Vec::new();
        let mut total_visual_count = 0;

        for line in buffer.iter().rev() {
            let line_visual_count = line.get_line_count();
            if total_visual_count + line_visual_count > max_visual_lines + state.scroll_offset {
                break;
            }
            visual_lines.insert(0, line.clone());
            total_visual_count += line_visual_count;
        }

        visual_lines
    }

    pub fn update_input(&self, input: String, cursor_pos: usize) {
        let mut state = self.state.borrow_mut();
        state.current_input = input;
        state.cursor_position = cursor_pos.min(state.current_input.chars().count());
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
        let mut state = self.state.borrow_mut();
        let buffer = self.buffer.borrow();
        let max_scroll = *self.max_scroll_lines.borrow();
        let terminal_height = *self.terminal_height.borrow();

        let total_visual_lines: usize = buffer.iter().map(|line| line.get_line_count()).sum();

        let max_scroll_offset = if total_visual_lines > terminal_height {
            (total_visual_lines - terminal_height).min(max_scroll)
        } else {
            0
        };

        let new_offset = (state.scroll_offset + lines).min(max_scroll_offset);

        if new_offset != state.scroll_offset {
            state.scroll_offset = new_offset;
            true
        } else {
            false
        }
    }

    pub fn scroll_down(&self, lines: usize) -> bool {
        let mut state = self.state.borrow_mut();
        let new_offset = state.scroll_offset.saturating_sub(lines);

        if new_offset != state.scroll_offset {
            state.scroll_offset = new_offset;
            true
        } else {
            false
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

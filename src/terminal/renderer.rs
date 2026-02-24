use super::buffer::{self, BufferLine, InputMode, LineType, TerminalState};
use super::linkmap::LinkMap;
use js_sys::Promise;
use std::cell::{Cell, RefCell};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{window, CanvasRenderingContext2d, HtmlCanvasElement};

#[derive(Default)]
pub struct LineOptions {
    pub color: Option<String>,
    pub boot_animation: bool,
    pub typing_speed: Option<i32>,
}

impl LineOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_color(mut self, color: &str) -> Self {
        self.color = Some(color.to_string());
        self
    }

    pub fn with_animation(mut self) -> Self {
        self.boot_animation = true;
        self
    }

    pub fn with_typing(mut self, speed: i32) -> Self {
        self.typing_speed = Some(speed);
        self
    }
}

pub struct TerminalRenderer {
    pub canvas: HtmlCanvasElement,
    pub context: CanvasRenderingContext2d,
    pub y: Cell<f64>,
    pub width: Cell<i32>,
    pub height: Cell<i32>,
    pub line_height: f64,
    pub char_width: f64,
    pub font_size: i32,
    pub cursor_blink_state: Cell<bool>,
    linkmap: RefCell<LinkMap>,
}

impl TerminalRenderer {
    pub fn new(canvas: HtmlCanvasElement, context: CanvasRenderingContext2d) -> Self {
        let width = canvas.width() as i32;
        let height = canvas.height() as i32;
        let font_size = 14;
        let line_height = font_size as f64 + 6.0;

        context.set_font(&format!("{}px 'Courier New', monospace", font_size));
        context.set_text_baseline("top");
        context.set_image_smoothing_enabled(false);

        let char_width = context
            .measure_text("M")
            .ok()
            .map(|metrics| metrics.width())
            .filter(|width| *width > 0.0)
            .unwrap_or(font_size as f64 * 0.6);

        Self {
            canvas,
            context,
            y: Cell::new(20.0),
            width: Cell::new(width),
            height: Cell::new(height),
            line_height,
            char_width,
            font_size,
            cursor_blink_state: Cell::new(true),
            linkmap: RefCell::new(LinkMap::new()),
        }
    }

    pub fn set_canvas_dimensions(&self, width: i32, height: i32) {
        if width <= 0 || height <= 0 {
            return;
        }

        self.width.set(width);
        self.height.set(height);
        self.y.set(20.0);
        buffer::set_terminal_dimensions(self.max_chars_per_line(), self.max_visible_lines());
    }

    fn sync_dimensions_from_canvas(&self) {
        let canvas_width = self.canvas.width() as i32;
        let canvas_height = self.canvas.height() as i32;

        if canvas_width != self.width.get() || canvas_height != self.height.get() {
            self.set_canvas_dimensions(canvas_width, canvas_height);
        }
    }

    pub async fn add_line(&self, text: &str, options: Option<LineOptions>) {
        let opts = options.unwrap_or_default();

        if opts.boot_animation {
            self.boot(text, &opts).await;
        } else if let Some(speed) = opts.typing_speed {
            self.typing(text, speed, &opts).await;
        } else {
            self.simple(text, &opts).await;
        }
    }

    async fn boot(&self, task: &str, opts: &LineOptions) {
        buffer::set_input_mode(InputMode::Disabled);
        let y = self.y.get();

        let spinner = ["⠋", "⠙", "⠹", "⠸"];
        for &spin in &spinner {
            let text = format!("{} {}", task, spin);
            self.clear_line_at_y(y);
            self.draw_text(&text, 10.0, y, opts.color.as_deref());
            self.sleep(60).await;
        }

        let final_text = format!("{} [OK]", task);
        self.clear_line_at_y(y);
        self.draw_boot_line(&final_text, y, opts.color.as_deref());
        buffer::add_line(final_text, LineType::Boot, opts.color.clone());
        self.advance_y();
        self.handle_scroll_if_needed();
    }

    async fn typing(&self, text: &str, speed: i32, opts: &LineOptions) {
        buffer::set_input_mode(InputMode::Disabled);
        let y = self.y.get();
        let mut displayed = String::new();

        for ch in text.chars() {
            displayed.push(ch);
            self.clear_line_at_y(y);
            self.draw_text(&displayed, 10.0, y, opts.color.as_deref());
            self.sleep(speed).await;
        }

        buffer::add_line(text.to_string(), LineType::Typing, opts.color.clone());
        self.advance_y();
        self.handle_scroll_if_needed();
    }

    async fn simple(&self, text: &str, opts: &LineOptions) {
        let y = self.y.get();
        buffer::add_line(text.to_string(), LineType::Normal, opts.color.clone());
        self.draw_text(text, 10.0, y, opts.color.as_deref());
        self.advance_y();
        self.handle_scroll_if_needed();
    }

    pub fn clear_screen(&self) {
        self.context.save();
        self.set_fill_color("#000000");
        let width = self.width.get() as f64;
        let height = self.height.get() as f64;
        self.context.fill_rect(0.0, 0.0, width, height);
        self.context.restore();
        self.y.set(20.0);
        self.linkmap.borrow_mut().clear();
    }

    pub fn max_visible_lines(&self) -> usize {
        ((self.height.get() as f64 - 40.0) / self.line_height) as usize
    }

    pub fn max_chars_per_line(&self) -> usize {
        ((self.width.get() as f64 - 20.0) / self.char_width) as usize
    }

    pub fn render(&self) {
        self.sync_dimensions_from_canvas();
        self.clear_screen();
        buffer::set_terminal_dimensions(self.max_chars_per_line(), self.max_visible_lines());
        let state = buffer::get_terminal_state();
        let reserved_input_lines = if state.input_mode == InputMode::Normal {
            self.estimate_input_line_count(&state)
        } else {
            0
        };

        let output_capacity = self
            .max_visible_lines()
            .saturating_sub(reserved_input_lines.saturating_add(1));
        let visible_lines = buffer::get_visible_lines(output_capacity);

        let mut y_offset = 20.0;
        for line in visible_lines {
            y_offset += self.render_line(&line, y_offset);
        }

        self.y.set(y_offset);

        if state.input_mode == InputMode::Normal {
            self.render_input_line(&state, y_offset);
        }
    }

    fn estimate_input_line_count(&self, state: &TerminalState) -> usize {
        let max_chars = self.max_chars_per_line().max(1);
        let prompt_chars = state.prompt.chars().count();
        let first_line_capacity = max_chars.saturating_sub(prompt_chars).max(1);
        let input_chars = state.current_input.chars().count();

        if input_chars <= first_line_capacity {
            1
        } else {
            1 + (input_chars - first_line_capacity + max_chars - 1) / max_chars
        }
    }

    fn render_line(&self, line: &BufferLine, y: f64) -> f64 {
        if line.line_type == LineType::Boot {
            self.draw_boot_line(&line.content, y, line.color.as_deref());
            if line.wrapped_lines.is_empty() {
                self.line_height
            } else {
                self.line_height * line.wrapped_lines.len() as f64
            }
        } else {
            let color = self.get_color(&line.line_type, line.color.as_deref());

            if line.wrapped_lines.is_empty() {
                self.draw_text(&line.content, 10.0, y, Some(&color));
                self.line_height
            } else {
                let mut current_y = y;
                for wrapped_line in &line.wrapped_lines {
                    self.draw_text(wrapped_line, 10.0, current_y, Some(&color));
                    current_y += self.line_height;
                }
                self.line_height * line.wrapped_lines.len() as f64
            }
        }
    }

    fn render_input_line(&self, state: &TerminalState, y: f64) {
        let max_chars = self.max_chars_per_line().max(1);
        let prompt_chars = state.prompt.chars().count();
        let first_line_capacity = max_chars.saturating_sub(prompt_chars).max(1);
        let input_chars: Vec<char> = state.current_input.chars().collect();

        let mut wrapped_input = Vec::new();

        if input_chars.is_empty() {
            wrapped_input.push(String::new());
        } else {
            let first_end = input_chars.len().min(first_line_capacity);
            wrapped_input.push(input_chars[..first_end].iter().collect());

            let mut index = first_end;
            while index < input_chars.len() {
                let end = (index + max_chars).min(input_chars.len());
                wrapped_input.push(input_chars[index..end].iter().collect());
                index = end;
            }
        }

        self.draw_text(&state.prompt, 10.0, y, Some("#00ffff"));

        let prompt_width = prompt_chars as f64 * self.char_width;
        let first_input_x = 10.0 + prompt_width;

        if let Some(first_line) = wrapped_input.first() {
            if !first_line.is_empty() {
                self.draw_text(first_line, first_input_x, y, Some("#ffffff"));
            }
        }

        let mut current_y = y + self.line_height;
        for line in wrapped_input.iter().skip(1) {
            if !line.is_empty() {
                self.draw_text(line, 10.0, current_y, Some("#ffffff"));
            }
            current_y += self.line_height;
        }

        let cursor_pos = state.cursor_position.min(input_chars.len());
        let (cursor_line, cursor_col) = if cursor_pos <= first_line_capacity {
            (0, cursor_pos)
        } else {
            let remaining = cursor_pos - first_line_capacity;
            (1 + (remaining / max_chars), remaining % max_chars)
        };

        if cursor_pos == input_chars.len()
            && !state.autosuggestion.is_empty()
            && state.autosuggestion.starts_with(&state.current_input)
            && cursor_line == 0
        {
            let suggestion_tail: String = state
                .autosuggestion
                .chars()
                .skip(input_chars.len())
                .collect();

            if !suggestion_tail.is_empty() {
                let suggestion_x = if cursor_line == 0 {
                    first_input_x + (cursor_col as f64 * self.char_width)
                } else {
                    10.0 + (cursor_col as f64 * self.char_width)
                };

                self.draw_text(&suggestion_tail, suggestion_x, y, Some("#5f6a70"));
            }
        }

        if self.cursor_blink_state.get() {
            let cursor_x = if cursor_line == 0 {
                first_input_x + (cursor_col as f64 * self.char_width)
            } else {
                10.0 + (cursor_col as f64 * self.char_width)
            };
            let cursor_y = y + (cursor_line as f64 * self.line_height);

            self.draw_cursor(cursor_x, cursor_y);
        }
    }

    pub fn draw_text(&self, text: &str, x: f64, y: f64, color: Option<&str>) {
        if text.contains('\u{1b}') {
            self.draw_ansi_text(text, x, y, color);
            return;
        }

        self.linkmap
            .borrow_mut()
            .detect_links(text, x, y, self.char_width, self.line_height);

        self.context.save();
        self.setup_font();

        let mut current_x = x;
        let mut pos = 0;

        while pos < text.len() {
            if let Some(http_start) = text[pos..].find("http") {
                let absolute_start = pos + http_start;

                let before_text = &text[pos..absolute_start];
                if !before_text.is_empty() {
                    self.set_fill_color(&self.get_color_value(color.unwrap_or("#ffffff")));
                    let _ = self.context.fill_text(before_text, current_x, y);
                    current_x += before_text.len() as f64 * self.char_width;
                }

                let remaining = &text[absolute_start..];
                let url_end = remaining
                    .find(|c: char| c.is_whitespace() || c == '\n' || c == ']')
                    .unwrap_or(remaining.len());
                let potential_url = &remaining[..url_end];

                if potential_url.starts_with("http://") || potential_url.starts_with("https://") {
                    self.set_fill_color("#00ffff");
                    let _ = self.context.fill_text(potential_url, current_x, y);

                    self.context.save();
                    self.context.set_stroke_style_str("#00ffff");
                    self.context.set_line_width(1.0);
                    self.context.begin_path();
                    self.context.move_to(current_x, y + self.line_height - 2.0);
                    self.context.line_to(
                        current_x + (potential_url.len() as f64 * self.char_width),
                        y + self.line_height - 2.0,
                    );
                    let _ = self.context.stroke();
                    self.context.restore();

                    current_x += potential_url.len() as f64 * self.char_width;
                    pos = absolute_start + url_end;
                } else {
                    self.set_fill_color(&self.get_color_value(color.unwrap_or("#ffffff")));
                    let _ = self.context.fill_text(
                        &text[absolute_start..absolute_start + 4],
                        current_x,
                        y,
                    );
                    current_x += 4.0 * self.char_width;
                    pos = absolute_start + 4;
                }
            } else {
                let remaining_text = &text[pos..];
                self.set_fill_color(&self.get_color_value(color.unwrap_or("#ffffff")));
                let _ = self.context.fill_text(remaining_text, current_x, y);
                break;
            }
        }

        self.context.restore();
    }

    fn draw_ansi_text(&self, text: &str, x: f64, y: f64, color: Option<&str>) {
        self.context.save();
        self.setup_font();

        let default_color = self.get_color_value(color.unwrap_or("#ffffff"));
        let mut current_color = default_color.clone();
        let mut current_x = x;
        let mut segment = String::new();

        let flush_segment =
            |renderer: &Self, segment: &mut String, x: &mut f64, y: f64, color: &str| {
                if segment.is_empty() {
                    return;
                }

                renderer.set_fill_color(color);
                let _ = renderer.context.fill_text(segment, *x, y);
                *x += segment.chars().count() as f64 * renderer.char_width;
                segment.clear();
            };

        let chars: Vec<char> = text.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            if chars[i] == '\u{1b}' && i + 1 < chars.len() && chars[i + 1] == '[' {
                flush_segment(self, &mut segment, &mut current_x, y, &current_color);

                i += 2;
                let mut code_buf = String::new();
                while i < chars.len() && chars[i] != 'm' {
                    code_buf.push(chars[i]);
                    i += 1;
                }

                if i < chars.len() && chars[i] == 'm' {
                    current_color = self.resolve_ansi_color(&code_buf, &default_color);
                }
            } else {
                segment.push(chars[i]);
            }

            i += 1;
        }

        flush_segment(self, &mut segment, &mut current_x, y, &current_color);
        self.context.restore();
    }

    fn resolve_ansi_color(&self, codes: &str, default_color: &str) -> String {
        if codes.is_empty() {
            return default_color.to_string();
        }

        let parts: Vec<&str> = codes.split(';').collect();
        let mut index = 0usize;
        let mut current = default_color.to_string();

        while index < parts.len() {
            let code = parts[index].parse::<i32>().unwrap_or(-1);

            match code {
                0 | 39 => current = default_color.to_string(),
                30 => current = "#000000".to_string(),
                31 => current = "#ff5555".to_string(),
                32 => current = "#50fa7b".to_string(),
                33 => current = "#f1fa8c".to_string(),
                34 => current = "#6272a4".to_string(),
                35 => current = "#ff79c6".to_string(),
                36 => current = "#8be9fd".to_string(),
                37 => current = "#f8f8f2".to_string(),
                90 => current = "#888888".to_string(),
                91 => current = "#ff6e6e".to_string(),
                92 => current = "#69ff94".to_string(),
                93 => current = "#ffffa5".to_string(),
                94 => current = "#8094d4".to_string(),
                95 => current = "#ff92df".to_string(),
                96 => current = "#a4ffff".to_string(),
                97 => current = "#ffffff".to_string(),
                38 => {
                    if index + 4 < parts.len() && parts[index + 1] == "2" {
                        let r = parts[index + 2].parse::<u8>().unwrap_or(255);
                        let g = parts[index + 3].parse::<u8>().unwrap_or(255);
                        let b = parts[index + 4].parse::<u8>().unwrap_or(255);
                        current = format!("#{:02x}{:02x}{:02x}", r, g, b);
                        index += 4;
                    }
                }
                _ => {}
            }

            index += 1;
        }

        current
    }

    pub fn draw_boot_line(&self, text: &str, y: f64, color: Option<&str>) {
        self.context.save();
        self.setup_font();

        if let Some(ok_pos) = text.rfind(" [OK]") {
            let main_text = &text[..ok_pos];
            let ok_text = " [OK]";

            self.set_fill_color(&self.get_color_value(color.unwrap_or("#ffffff")));
            self.context.fill_text(main_text, 10.0, y).unwrap();

            let main_width = main_text.len() as f64 * self.char_width;

            self.set_fill_color("#00ff00");
            self.context
                .fill_text(ok_text, 10.0 + main_width, y)
                .unwrap();
        } else {
            self.set_fill_color(&self.get_color_value(color.unwrap_or("#ffffff")));
            self.context.fill_text(text, 10.0, y).unwrap();
        }

        self.context.restore();
    }

    fn clear_line_at_y(&self, y: f64) {
        self.context.save();
        self.set_fill_color("#000000");
        let width = self.width.get() as f64;
        self.context.fill_rect(0.0, y, width, self.line_height);
        self.context.restore();
    }

    fn draw_cursor(&self, x: f64, y: f64) {
        self.context.save();
        self.set_fill_color("#ffffff");

        let cursor_height = self.line_height - 6.0;
        let cursor_y_offset = -1.0;

        self.context
            .fill_rect(x, y + cursor_y_offset, 2.0, cursor_height);
        self.context.restore();
    }

    fn set_fill_color(&self, color: &str) {
        let _ = js_sys::Reflect::set(
            &self.context,
            &JsValue::from_str("fillStyle"),
            &JsValue::from_str(color),
        );
    }

    fn setup_font(&self) {
        self.context.set_font("14px monospace");
        self.context.set_text_baseline("top");
    }

    fn get_color(&self, line_type: &LineType, custom_color: Option<&str>) -> String {
        if let Some(color) = custom_color {
            return self.get_color_value(color);
        }

        match line_type {
            LineType::Command => "#00ffff",
            LineType::Output => "#ffffff",
            LineType::Boot => "#ffffff",
            LineType::Typing => "#ffffff",
            LineType::Normal => "#ffffff",
        }
        .to_string()
    }

    pub fn get_color_value(&self, color: &str) -> String {
        match color {
            "red" => "#ff0000",
            "green" => "#00ff00",
            "blue" => "#0000ff",
            "yellow" => "#ffff00",
            "cyan" => "#00ffff",
            "magenta" => "#ff00ff",
            "white" => "#ffffff",
            "gray" | "grey" => "#808080",
            "boot-line" | "typing-line" => "#ffffff",
            "command" => "#8be9fd",
            "completion" => "#f8f8f2",
            "error" => "#ff4444",
            "success" => "#44ff44",
            "warning" => "#ffaa00",
            _ => {
                if color.starts_with('#') || color.starts_with("rgb") {
                    color
                } else {
                    "#ffffff"
                }
            }
        }
        .to_string()
    }

    fn advance_y(&self) {
        self.y.set(self.y.get() + self.line_height);
    }

    fn handle_scroll_if_needed(&self) {
        let max_lines = (self.height.get() as f64 / self.line_height) as i32;
        let current_line = ((self.y.get() - 20.0) / self.line_height) as i32;

        if current_line >= max_lines - 3 {
            self.render();
        }
    }

    pub fn clear_output(&self) {
        buffer::clear_buffer();
        self.clear_screen();
        self.prepare_for_input();
    }

    pub fn prepare_for_input(&self) {
        let prompt = "objz@portfolio:~$ ";
        buffer::set_current_prompt(prompt.to_string());
        buffer::set_input_mode(InputMode::Normal);
        buffer::update_input_state(String::new(), 0);
        buffer::auto_scroll_to_bottom();
        self.render();
    }

    pub fn toggle_cursor(&self) {
        self.cursor_blink_state.set(!self.cursor_blink_state.get());
    }

    pub fn show_cursor(&self) {
        self.cursor_blink_state.set(true);
    }

    pub fn hide_cursor(&self) {
        self.cursor_blink_state.set(false);
    }

    pub fn handle_click(&self, x: f64, y: f64) -> Option<String> {
        self.linkmap.borrow().find_link(x, y)
    }

    pub async fn sleep(&self, ms: i32) {
        let promise = Promise::new(&mut |resolve, _reject| {
            let window = window().unwrap();
            let closure = wasm_bindgen::prelude::Closure::once_into_js(move || {
                resolve.call0(&wasm_bindgen::JsValue::UNDEFINED).unwrap();
            });
            window
                .set_timeout_with_callback_and_timeout_and_arguments_0(
                    closure.as_ref().unchecked_ref(),
                    ms,
                )
                .unwrap();
        });

        let _ = JsFuture::from(promise).await;
    }
}

impl Clone for TerminalRenderer {
    fn clone(&self) -> Self {
        Self {
            canvas: self.canvas.clone(),
            context: self.context.clone(),
            y: Cell::new(self.y.get()),
            width: Cell::new(self.width.get()),
            height: Cell::new(self.height.get()),
            line_height: self.line_height,
            char_width: self.char_width,
            font_size: self.font_size,
            cursor_blink_state: Cell::new(self.cursor_blink_state.get()),
            linkmap: RefCell::new(LinkMap::new()),
        }
    }
}

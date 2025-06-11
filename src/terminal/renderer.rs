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

    pub fn with_boot_animation(mut self) -> Self {
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
    pub width: i32,
    pub height: i32,
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
        let char_width = font_size as f64 * 0.6;

        context.set_font(&format!("{}px 'Courier New', monospace", font_size));
        context.set_text_baseline("top");
        context.set_image_smoothing_enabled(false);

        Self {
            canvas,
            context,
            y: Cell::new(20.0),
            width,
            height,
            line_height,
            char_width,
            font_size,
            cursor_blink_state: Cell::new(true),
            linkmap: RefCell::new(LinkMap::new()),
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
        self.context
            .fill_rect(0.0, 0.0, self.width as f64, self.height as f64);
        self.context.restore();
        self.y.set(20.0);
        self.linkmap.borrow_mut().clear();
    }

    pub fn max_visible_lines(&self) -> usize {
        ((self.height as f64 - 40.0) / self.line_height) as usize
    }

    pub fn max_chars_per_line(&self) -> usize {
        ((self.width as f64 - 20.0) / self.char_width) as usize
    }

    pub fn render(&self) {
        self.clear_screen();
        buffer::set_terminal_dimensions(self.max_chars_per_line(), self.max_visible_lines());
        let visible_lines = buffer::get_visible_lines(self.max_visible_lines() - 2);
        let state = buffer::get_terminal_state();

        let mut y_offset = 20.0;
        for line in visible_lines {
            y_offset += self.render_line(&line, y_offset);
        }

        self.y.set(y_offset);

        if state.input_mode == InputMode::Normal {
            self.render_input_line(&state, y_offset);
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
        self.clear_line_at_y(y);
        self.draw_text(&state.prompt, 10.0, y, Some("#00ffff"));

        let prompt_width = state.prompt.len() as f64 * self.char_width;
        let input_x = 10.0 + prompt_width;

        if !state.current_input.is_empty() {
            self.draw_text(&state.current_input, input_x, y, Some("#ffffff"));
        }

        if self.cursor_blink_state.get() {
            let cursor_x = input_x + (state.cursor_position as f64 * self.char_width);
            self.draw_cursor(cursor_x, y);
        }
    }

    pub fn draw_text(&self, text: &str, x: f64, y: f64, color: Option<&str>) {
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
                    self.context
                        .set_stroke_style(&wasm_bindgen::JsValue::from_str("#00ffff"));
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
        self.context
            .fill_rect(0.0, y, self.width as f64, self.line_height);
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
            LineType::_Error => "#ff0000",
            LineType::System => "#ffff00",
            LineType::Boot => "#ffffff",
            LineType::Typing => "#ffffff",
            LineType::_Prompt => "#00ffff",
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
        let max_lines = (self.height as f64 / self.line_height) as i32;
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
            width: self.width,
            height: self.height,
            line_height: self.line_height,
            char_width: self.char_width,
            font_size: self.font_size,
            cursor_blink_state: Cell::new(self.cursor_blink_state.get()),
            linkmap: RefCell::new(LinkMap::new()),
        }
    }
}

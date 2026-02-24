use super::buffer;
use super::renderer::{LineOptions, TerminalRenderer};
use crate::commands::CommandHandler;
use js_sys::Promise;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{window, CanvasRenderingContext2d, Document, HtmlCanvasElement};

const TERMINAL_ASPECT_RATIO: f64 = 700.0 / 550.0;
const DEFAULT_CANVAS_WIDTH: f64 = 700.0;
const DEFAULT_CANVAS_HEIGHT: f64 = 550.0;

#[derive(Clone)]
pub struct Terminal {
    pub renderer: TerminalRenderer,
    pub command_handler: CommandHandler,
    pub base_prompt: String,
}

impl Terminal {
    fn compute_canvas_size() -> (f64, f64, f64) {
        let Some(window) = window() else {
            return (DEFAULT_CANVAS_WIDTH, DEFAULT_CANVAS_HEIGHT, 1.0);
        };

        let viewport_width = window
            .inner_width()
            .ok()
            .and_then(|value| value.as_f64())
            .unwrap_or(1280.0);
        let viewport_height = window
            .inner_height()
            .ok()
            .and_then(|value| value.as_f64())
            .unwrap_or(720.0);

        let desktop_layout = viewport_width >= 1100.0 && viewport_height >= 720.0;

        let (css_width, css_height) = if desktop_layout {
            (DEFAULT_CANVAS_WIDTH, DEFAULT_CANVAS_HEIGHT)
        } else {
            let width_budget = (viewport_width * 0.88).clamp(320.0, DEFAULT_CANVAS_WIDTH);
            let height_budget = (viewport_height * 0.56).clamp(240.0, DEFAULT_CANVAS_HEIGHT);

            let mut width = width_budget;
            let mut height = width / TERMINAL_ASPECT_RATIO;

            if height > height_budget {
                height = height_budget;
                width = height * TERMINAL_ASPECT_RATIO;
            }

            (width.max(320.0), height.max(240.0))
        };

        let dpr = window.device_pixel_ratio().clamp(1.0, 2.0);

        (css_width, css_height, dpr)
    }

    fn apply_canvas_size(canvas: &HtmlCanvasElement) {
        let (css_width, css_height, dpr) = Self::compute_canvas_size();
        let pixel_width = (css_width * dpr).round().max(320.0) as u32;
        let pixel_height = (css_height * dpr).round().max(220.0) as u32;

        canvas.set_width(pixel_width);
        canvas.set_height(pixel_height);

        let style = canvas.style();
        let _ = style.set_property("width", &format!("{:.0}px", css_width));
        let _ = style.set_property("height", &format!("{:.0}px", css_height));
    }

    pub fn new(document: &Document) -> Self {
        let canvas = document
            .get_element_by_id("terminal")
            .expect("canvas not found")
            .dyn_into::<HtmlCanvasElement>()
            .expect("element is not a canvas");

        Self::apply_canvas_size(&canvas);

        let context = canvas
            .get_context("2d")
            .expect("failed to get 2d context")
            .unwrap()
            .dyn_into::<CanvasRenderingContext2d>()
            .expect("failed to cast to CanvasRenderingContext2d");

        let renderer = TerminalRenderer::new(canvas.clone(), context);
        renderer.set_canvas_dimensions(canvas.width() as i32, canvas.height() as i32);
        let command_handler = CommandHandler::new();
        let base_prompt = "objz@portfolio".to_string();

        buffer::set_terminal_dimensions(
            renderer.max_chars_per_line(),
            renderer.max_visible_lines(),
        );

        let terminal = Self {
            renderer,
            command_handler,
            base_prompt,
        };

        terminal.setup_events(&canvas);
        terminal
    }

    fn setup_events(&self, canvas: &HtmlCanvasElement) {
        let renderer_clone = self.renderer.clone();
        let mousemove_closure = Closure::wrap(Box::new(move |event: web_sys::MouseEvent| {
            let rect = renderer_clone.canvas.get_bounding_client_rect();
            let x = event.client_x() as f64 - rect.left();
            let y = event.client_y() as f64 - rect.top();

            let cursor = if renderer_clone.handle_click(x, y).is_some() {
                "pointer"
            } else {
                "default"
            };

            let style = renderer_clone.canvas.style();
            let _ = style.set_property("cursor", cursor);
        }) as Box<dyn FnMut(_)>);

        let canvas_el = canvas.clone();
        canvas_el.set_attribute("tabindex", "0").unwrap();

        let _ = canvas_el.add_event_listener_with_callback_and_add_event_listener_options(
            "mousemove",
            mousemove_closure.as_ref().unchecked_ref(),
            &web_sys::AddEventListenerOptions::new(),
        );

        mousemove_closure.forget();

        let click_renderer = self.renderer.clone();
        let click_closure = Closure::wrap(Box::new(move |event: web_sys::MouseEvent| {
            let rect = click_renderer.canvas.get_bounding_client_rect();
            let x = event.client_x() as f64 - rect.left();
            let y = event.client_y() as f64 - rect.top();

            if let Some(url) = click_renderer.handle_click(x, y) {
                if let Some(window) = window() {
                    let _ = window.open_with_url_and_target(&url, "_blank");
                }
            }
        }) as Box<dyn FnMut(_)>);

        let _ = canvas_el
            .add_event_listener_with_callback("click", click_closure.as_ref().unchecked_ref());

        click_closure.forget();

        let terminal_clone = self.clone();
        let resize_closure = Closure::wrap(Box::new(move |_event: web_sys::Event| {
            terminal_clone.resize_canvas_to_viewport();
            terminal_clone.render();
        }) as Box<dyn FnMut(_)>);

        if let Some(window) = window() {
            let _ = window.add_event_listener_with_callback(
                "resize",
                resize_closure.as_ref().unchecked_ref(),
            );
        }

        resize_closure.forget();
    }

    pub fn resize_canvas_to_viewport(&self) {
        Self::apply_canvas_size(&self.renderer.canvas);
        self.renderer.set_canvas_dimensions(
            self.renderer.canvas.width() as i32,
            self.renderer.canvas.height() as i32,
        );
    }

    pub fn get_current_prompt(&self) -> String {
        let cwd = self.command_handler.get_working_dir();
        let display_path = if cwd == "/home/objz" {
            "~".to_string()
        } else if cwd.starts_with("/home/objz/") {
            format!("~{}", &cwd["/home/objz".len()..])
        } else {
            cwd
        };

        format!("{}:{}$ ", self.base_prompt, display_path)
    }

    pub async fn sleep(&self, ms: i32) {
        let promise = Promise::new(&mut |resolve, _reject| {
            let window = window().unwrap();
            let closure = Closure::once_into_js(move || {
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

    pub async fn add_line(&self, text: &str, options: Option<LineOptions>) {
        self.renderer.add_line(text, options).await;
    }

    pub fn clear_output(&self) {
        self.renderer.clear_output();
    }

    pub fn prepare_for_input(&self) {
        let prompt = self.get_current_prompt();
        buffer::set_current_prompt(prompt);
        self.renderer.prepare_for_input();
    }

    pub fn render(&self) {
        self.renderer.render();
    }
}

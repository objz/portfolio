use super::buffer;
use super::renderer::{LineOptions, TerminalRenderer};
use crate::commands::CommandHandler;
use js_sys::Promise;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{window, CanvasRenderingContext2d, Document, HtmlCanvasElement};

#[derive(Clone)]
pub struct Terminal {
    pub renderer: TerminalRenderer,
    pub command_handler: CommandHandler,
    pub base_prompt: String,
}

impl Terminal {
    pub fn new(document: &Document) -> Self {
        let canvas = document
            .get_element_by_id("terminal")
            .expect("canvas not found")
            .dyn_into::<HtmlCanvasElement>()
            .expect("element is not a canvas");

        let canvas_width = 700;
        let canvas_height = 550;

        canvas.set_width(canvas_width);
        canvas.set_height(canvas_height);

        let context = canvas
            .get_context("2d")
            .expect("failed to get 2d context")
            .unwrap()
            .dyn_into::<CanvasRenderingContext2d>()
            .expect("failed to cast to CanvasRenderingContext2d");

        let renderer = TerminalRenderer::new(canvas.clone(), context);
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

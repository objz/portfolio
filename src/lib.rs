use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::{cell::RefCell, rc::Rc};
use web_sys::Event;

use input::setup::InputHandler;
use wasm_bindgen::prelude::*;
use web_sys::HtmlInputElement;

mod ascii;
mod boot;
mod commands;
mod github;
mod input;
mod shell;
mod terminal;
mod utils;

use terminal::Terminal;

static TERMINAL_READY: AtomicBool = AtomicBool::new(false);

thread_local! {
    static TERMINAL_INSTANCE: RefCell<Option<Terminal>> = RefCell::new(None);
}

#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();

    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("should have a document on window");

    commands::system::init();

    let terminal = Terminal::new(&document);

    let hidden_input = document
        .get_element_by_id("hidden-input")
        .expect("hidden input not found")
        .dyn_into::<HtmlInputElement>()
        .expect("element is not an input");

    InputHandler::setup(&terminal, &hidden_input);

    TERMINAL_INSTANCE.with(|instance| {
        *instance.borrow_mut() = Some(terminal);
    });

    let boot_started = Rc::new(AtomicBool::new(false));
    let boot_started_clone = boot_started.clone();

    let closure = Closure::wrap(Box::new(move |_event: Event| {
        TERMINAL_READY.store(true, Ordering::SeqCst);

        if boot_started_clone.swap(true, Ordering::SeqCst) {
            return;
        }

        TERMINAL_INSTANCE.with(|instance| {
            let terminal = instance.borrow().clone();

            if let Some(terminal) = terminal {
                wasm_bindgen_futures::spawn_local(async move {
                    terminal.init_boot().await;
                });
            }
        });
    }) as Box<dyn FnMut(_)>);

    window
        .add_event_listener_with_callback("terminalReady", closure.as_ref().unchecked_ref())
        .expect("Failed to add event listener");

    closure.forget();
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[wasm_bindgen]
pub fn greet() {
    log("Terminal loaded");
}

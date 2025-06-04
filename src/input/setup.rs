use crate::commands::CommandHandler;
use crate::input::history::CommandHistory;
use crate::terminal::autocomplete::{find_common_prefix, AutoComplete, CompletionResult};
use crate::terminal::buffer::{self, InputMode};
use crate::terminal::Terminal;
use crate::utils::panic;
use std::cell::RefCell;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::{window, HtmlInputElement, KeyboardEvent};

thread_local! {
    static CURRENT_INPUT: RefCell<String> = RefCell::new(String::new());
    static IS_FOCUSED: RefCell<bool> = RefCell::new(false);
    static AUTOCOMPLETE: RefCell<AutoComplete> = RefCell::new(AutoComplete::new());
}

pub struct InputHandler;

impl InputHandler {
    pub fn setup(terminal: &Terminal, hidden_input: &HtmlInputElement) {
        let history = CommandHistory::new();
        let processor = terminal.command_handler.clone();

        let terminal_clone = terminal.clone();
        let hidden_input_clone = hidden_input.clone();

        let input_callback = {
            let terminal = terminal_clone.clone();
            let hidden_input = hidden_input_clone.clone();

            Closure::wrap(Box::new(move |_event: web_sys::Event| {
                let current_value = hidden_input.value();
                CURRENT_INPUT.with(|input| {
                    *input.borrow_mut() = current_value.clone();
                });

                let cursor_pos = hidden_input
                    .selection_start()
                    .unwrap_or(Some(0))
                    .unwrap_or(0) as usize;

                buffer::update_input_state(current_value, cursor_pos);
                terminal.render();
            }) as Box<dyn FnMut(_)>)
        };

        hidden_input
            .add_event_listener_with_callback("input", input_callback.as_ref().unchecked_ref())
            .unwrap();
        input_callback.forget();

        let keydown_callback = {
            let terminal = terminal_clone.clone();
            let hidden_input = hidden_input_clone.clone();
            let history = RefCell::new(history);
            let processor = RefCell::new(processor);

            Closure::wrap(Box::new(move |event: KeyboardEvent| {
                let current_input = CURRENT_INPUT.with(|input| input.borrow().clone());

                match event.key().as_str() {
                    "Enter" => {
                        event.prevent_default();
                        Self::handle_enter(
                            &current_input,
                            &mut history.borrow_mut(),
                            &mut processor.borrow_mut(),
                            &terminal,
                            &hidden_input,
                        );
                    }
                    "ArrowUp" => {
                        event.prevent_default();
                        if let Some(cmd) = history.borrow_mut().prev() {
                            hidden_input.set_value(cmd);
                            CURRENT_INPUT.with(|input| {
                                *input.borrow_mut() = cmd.clone();
                            });
                            buffer::update_input_state(cmd.clone(), cmd.len());
                            terminal.render();
                        }
                    }
                    "ArrowDown" => {
                        event.prevent_default();
                        if let Some(cmd) = history.borrow_mut().next() {
                            hidden_input.set_value(cmd);
                            CURRENT_INPUT.with(|input| {
                                *input.borrow_mut() = cmd.clone();
                            });
                            buffer::update_input_state(cmd.clone(), cmd.len());
                        } else {
                            hidden_input.set_value("");
                            CURRENT_INPUT.with(|input| {
                                input.borrow_mut().clear();
                            });
                            buffer::update_input_state(String::new(), 0);
                        }
                        terminal.render();
                    }
                    "ArrowLeft" => {
                        event.prevent_default();
                        let current_cursor = hidden_input
                            .selection_start()
                            .unwrap_or(Some(0))
                            .unwrap_or(0) as usize;

                        if current_cursor > 0 {
                            let new_cursor = current_cursor - 1;
                            let _ = hidden_input
                                .set_selection_range(new_cursor as u32, new_cursor as u32);
                            buffer::update_input_state(current_input, new_cursor);
                            terminal.render();
                        }
                    }
                    "ArrowRight" => {
                        event.prevent_default();
                        let current_cursor = hidden_input
                            .selection_start()
                            .unwrap_or(Some(0))
                            .unwrap_or(0) as usize;

                        let input_len = current_input.len();
                        if current_cursor < input_len {
                            let new_cursor = current_cursor + 1;
                            let _ = hidden_input
                                .set_selection_range(new_cursor as u32, new_cursor as u32);
                            buffer::update_input_state(current_input, new_cursor);
                            terminal.render();
                        }
                    }
                    "Home" => {
                        event.prevent_default();
                        let _ = hidden_input.set_selection_range(0, 0);
                        buffer::update_input_state(current_input, 0);
                        terminal.render();
                    }
                    "End" => {
                        event.prevent_default();
                        let input_len = current_input.len();
                        let cursor_pos = input_len as u32;
                        let _ = hidden_input.set_selection_range(cursor_pos, cursor_pos);
                        buffer::update_input_state(current_input, input_len);
                        terminal.render();
                    }
                    "Tab" => {
                        event.prevent_default();
                        Self::tab_complete(&terminal, &hidden_input, &current_input);
                    }
                    _ => {}
                }
            }) as Box<dyn FnMut(_)>)
        };

        hidden_input
            .add_event_listener_with_callback("keydown", keydown_callback.as_ref().unchecked_ref())
            .unwrap();
        keydown_callback.forget();

        Self::setup_focus_listeners(&terminal_clone, &hidden_input_clone);
        Self::setup_cursor_blink(&terminal_clone);
        Self::setup_custom_listeners(&terminal_clone);
        Self::setup_scroll_listeners(&terminal_clone);

        terminal.prepare_for_input();
        let _ = hidden_input.focus();
    }

    fn setup_focus_listeners(terminal: &Terminal, hidden_input: &HtmlInputElement) {
        let terminal_clone = terminal.clone();

        let focus_callback = {
            let terminal = terminal_clone.clone();
            Closure::wrap(Box::new(move |_event: web_sys::Event| {
                IS_FOCUSED.with(|focused| {
                    *focused.borrow_mut() = true;
                });
                terminal.renderer.show_cursor();
                terminal.render();
            }) as Box<dyn FnMut(_)>)
        };

        hidden_input
            .add_event_listener_with_callback("focus", focus_callback.as_ref().unchecked_ref())
            .unwrap();
        focus_callback.forget();

        let blur_callback = {
            let terminal = terminal_clone.clone();
            Closure::wrap(Box::new(move |_event: web_sys::Event| {
                IS_FOCUSED.with(|focused| {
                    *focused.borrow_mut() = false;
                });
                terminal.renderer.hide_cursor();
                terminal.render();
            }) as Box<dyn FnMut(_)>)
        };

        hidden_input
            .add_event_listener_with_callback("blur", blur_callback.as_ref().unchecked_ref())
            .unwrap();
        blur_callback.forget();
    }

    fn setup_custom_listeners(terminal: &Terminal) {
        let terminal_clone = terminal.clone();
        let window = window().unwrap();

        let focus_event_callback = {
            let terminal = terminal_clone.clone();
            Closure::wrap(Box::new(move |_event: web_sys::Event| {
                IS_FOCUSED.with(|focused| {
                    *focused.borrow_mut() = true;
                });
                terminal.renderer.show_cursor();
                terminal.render();
            }) as Box<dyn FnMut(_)>)
        };

        window
            .add_event_listener_with_callback(
                "terminalFocus",
                focus_event_callback.as_ref().unchecked_ref(),
            )
            .unwrap();
        focus_event_callback.forget();

        let blur_event_callback = {
            let terminal = terminal_clone.clone();
            Closure::wrap(Box::new(move |_event: web_sys::Event| {
                IS_FOCUSED.with(|focused| {
                    *focused.borrow_mut() = false;
                });
                terminal.renderer.hide_cursor();
                terminal.render();
            }) as Box<dyn FnMut(_)>)
        };

        window
            .add_event_listener_with_callback(
                "terminalBlur",
                blur_event_callback.as_ref().unchecked_ref(),
            )
            .unwrap();
        blur_event_callback.forget();
    }

    fn setup_scroll_listeners(terminal: &Terminal) {
        let terminal_clone = terminal.clone();
        let window = window().unwrap();

        let scroll_up_callback = {
            let terminal = terminal_clone.clone();
            Closure::wrap(Box::new(move |_event: web_sys::Event| {
                if buffer::scroll_up(3) {
                    terminal.render();
                }
            }) as Box<dyn FnMut(_)>)
        };

        window
            .add_event_listener_with_callback(
                "terminalScrollUp",
                scroll_up_callback.as_ref().unchecked_ref(),
            )
            .unwrap();
        scroll_up_callback.forget();

        let scroll_down_callback = {
            let terminal = terminal_clone.clone();
            Closure::wrap(Box::new(move |_event: web_sys::Event| {
                if buffer::scroll_down(3) {
                    terminal.render();
                }
            }) as Box<dyn FnMut(_)>)
        };

        window
            .add_event_listener_with_callback(
                "terminalScrollDown",
                scroll_down_callback.as_ref().unchecked_ref(),
            )
            .unwrap();
        scroll_down_callback.forget();

        let scroll_to_bottom_callback = {
            let terminal = terminal_clone.clone();
            Closure::wrap(Box::new(move |_event: web_sys::Event| {
                buffer::reset_scroll();
                terminal.render();
            }) as Box<dyn FnMut(_)>)
        };

        window
            .add_event_listener_with_callback(
                "terminalScrollToBottom",
                scroll_to_bottom_callback.as_ref().unchecked_ref(),
            )
            .unwrap();
        scroll_to_bottom_callback.forget();
    }

    fn handle_enter(
        current_input: &str,
        history: &mut CommandHistory,
        processor: &mut CommandHandler,
        terminal: &Terminal,
        hidden_input: &HtmlInputElement,
    ) {
        let trimmed_input = current_input.trim();

        if panic::should_panic(trimmed_input) {
            history.add(trimmed_input.to_string());
            let prompt = terminal.get_current_prompt();
            buffer::add_command_line(&prompt, trimmed_input);

            hidden_input.set_value("");
            CURRENT_INPUT.with(|input| input.borrow_mut().clear());
            buffer::update_input_state(String::new(), 0);
            buffer::set_input_mode(InputMode::Processing);

            let terminal_clone = terminal.clone();
            let hidden_input_clone = hidden_input.clone();
            spawn_local(async move {
                panic::trigger(&terminal_clone).await;
                Self::prepare_input(&terminal_clone, &hidden_input_clone);
            });
            return;
        }

        if !trimmed_input.is_empty() {
            history.add(trimmed_input.to_string());
            let prompt = terminal.get_current_prompt();
            buffer::add_command_line(&prompt, trimmed_input);
        }

        hidden_input.set_value("");
        CURRENT_INPUT.with(|input| input.borrow_mut().clear());
        buffer::update_input_state(String::new(), 0);
        buffer::set_input_mode(InputMode::Processing);

        if !trimmed_input.is_empty() {
            let (result, _directory_changed) = processor.handle(trimmed_input);

            match result.as_str() {
                "CLEAR_SCREEN" => {
                    buffer::clear_buffer();
                    Self::prepare_input(terminal, hidden_input);
                }
                "SYSTEM_PANIC" => {
                    let terminal_clone = terminal.clone();
                    let hidden_input_clone = hidden_input.clone();
                    spawn_local(async move {
                        panic::trigger(&terminal_clone).await;
                        Self::prepare_input(&terminal_clone, &hidden_input_clone);
                    });
                }
                _ => {
                    if !result.is_empty() {
                        buffer::add_output_lines(&result, None);
                    }
                    Self::prepare_input(terminal, hidden_input);
                }
            }
        } else {
            Self::prepare_input(terminal, hidden_input);
        }
    }

    fn prepare_input(terminal: &Terminal, hidden_input: &HtmlInputElement) {
        let prompt = terminal.get_current_prompt();
        buffer::set_current_prompt(prompt);
        buffer::set_input_mode(InputMode::Normal);
        buffer::auto_scroll_to_bottom();

        terminal.render();
        let _ = hidden_input.focus();
    }

    fn tab_complete(terminal: &Terminal, hidden_input: &HtmlInputElement, current_input: &str) {
        let current_path = {
            use crate::commands::filesystem::CURRENT_PATH;
            CURRENT_PATH.lock().unwrap().clone()
        };

        let trimmed = current_input.trim();
        let parts: Vec<&str> = trimmed.split_whitespace().collect();

        let (command_prefix, completion_target) = if parts.is_empty() {
            ("", current_input)
        } else if parts.len() == 1 && !trimmed.ends_with(' ') {
            ("", current_input)
        } else {
            if parts.len() == 1 {
                (trimmed, "")
            } else {
                let last_space_idx = current_input.rfind(' ').unwrap_or(0);
                let prefix = &current_input[..=last_space_idx];
                let target = &current_input[last_space_idx + 1..];
                (prefix, target)
            }
        };

        let completion_result = AUTOCOMPLETE.with(|autocomplete| {
            autocomplete
                .borrow_mut()
                .complete(current_input, &current_path)
        });

        match completion_result {
            CompletionResult::None => {}
            CompletionResult::Single(completion) => {
                let full_completion = if command_prefix.is_empty() {
                    completion
                } else {
                    format!("{}{}", command_prefix, completion)
                };

                hidden_input.set_value(&full_completion);
                CURRENT_INPUT.with(|input| {
                    *input.borrow_mut() = full_completion.clone();
                });

                let cursor_pos = full_completion.len();
                let _ = hidden_input.set_selection_range(cursor_pos as u32, cursor_pos as u32);

                buffer::update_input_state(full_completion, cursor_pos);
                terminal.render();
            }
            CompletionResult::Multiple(completions) => {
                if let Some(common_prefix) = find_common_prefix(&completions) {
                    if common_prefix.len() > completion_target.len() {
                        let full_completion = if command_prefix.is_empty() {
                            common_prefix
                        } else {
                            format!("{}{}", command_prefix, common_prefix)
                        };

                        hidden_input.set_value(&full_completion);
                        CURRENT_INPUT.with(|input| {
                            *input.borrow_mut() = full_completion.clone();
                        });

                        let cursor_pos = full_completion.len();
                        let _ =
                            hidden_input.set_selection_range(cursor_pos as u32, cursor_pos as u32);

                        buffer::update_input_state(full_completion, cursor_pos);
                        terminal.render();
                        return;
                    }
                }

                let prompt = terminal.get_current_prompt();
                buffer::add_command_line(&prompt, current_input);

                let completions_text = if completions.len() <= 10 {
                    completions.join("  ")
                } else {
                    let mut output = String::new();
                    for (i, completion) in completions.iter().enumerate() {
                        if i > 0 && i % 4 == 0 {
                            output.push('\n');
                        } else if i > 0 {
                            output.push_str("  ");
                        }
                        output.push_str(completion);
                    }
                    output
                };

                buffer::add_output_lines(&completions_text, None);

                Self::prepare_input(terminal, hidden_input);
                hidden_input.set_value(current_input);
                CURRENT_INPUT.with(|input| {
                    *input.borrow_mut() = current_input.to_string();
                });

                let cursor_pos = current_input.len();
                let _ = hidden_input.set_selection_range(cursor_pos as u32, cursor_pos as u32);

                buffer::update_input_state(current_input.to_string(), cursor_pos);
                terminal.render();
            }
        }
    }

    fn setup_cursor_blink(terminal: &Terminal) {
        let terminal_clone = terminal.clone();

        let blink_callback = Closure::wrap(Box::new(move || {
            let is_focused = IS_FOCUSED.with(|focused| *focused.borrow());
            let state = buffer::get_terminal_state();

            if is_focused && state.input_mode == InputMode::Normal {
                terminal_clone.renderer.toggle_cursor();
                terminal_clone.render();
            }
        }) as Box<dyn FnMut()>);

        window()
            .unwrap()
            .set_interval_with_callback_and_timeout_and_arguments_0(
                blink_callback.as_ref().unchecked_ref(),
                500,
            )
            .unwrap();
        blink_callback.forget();
    }
}

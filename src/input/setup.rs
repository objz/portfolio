use crate::commands::registry;
use crate::commands::CommandHandler;
use crate::input::editor::{self, EditorEvent};
use crate::input::history::CommandHistory;
use crate::terminal::autocomplete::{AutoComplete, CompletionResult};
use crate::terminal::buffer::{self, InputMode};
use crate::terminal::Terminal;
use crate::utils::panic;
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::{window, CustomEvent, HtmlInputElement, KeyboardEvent};

thread_local! {
    static CURRENT_INPUT: RefCell<String> = const { RefCell::new(String::new()) };
    static IS_FOCUSED: RefCell<bool> = const { RefCell::new(false) };
    static AUTOCOMPLETE: RefCell<AutoComplete> = RefCell::new(AutoComplete::new());
}

pub struct InputHandler;

impl InputHandler {
    pub fn setup(terminal: &Terminal, hidden_input: &HtmlInputElement) {
        let history = Rc::new(RefCell::new(CommandHistory::new()));
        let processor = Rc::new(RefCell::new(terminal.command_handler.clone()));

        let terminal_clone = terminal.clone();
        let hidden_input_clone = hidden_input.clone();

        let input_callback = {
            let terminal = terminal_clone.clone();
            let hidden_input = hidden_input_clone.clone();
            let history = history.clone();

            Closure::wrap(Box::new(move |_event: web_sys::Event| {
                if editor::is_active() {
                    hidden_input.set_value("");
                    return;
                }

                let state = buffer::get_terminal_state();
                if state.input_mode == InputMode::Disabled {
                    return;
                }

                history.borrow_mut().reset_navigation();
                let current_value = hidden_input.value();
                CURRENT_INPUT.with(|input| {
                    *input.borrow_mut() = current_value.clone();
                });

                let cursor_pos = hidden_input
                    .selection_start()
                    .unwrap_or(Some(0))
                    .unwrap_or(0) as usize;

                let current_path = {
                    use crate::commands::filesystem::CURRENT_PATH;
                    CURRENT_PATH.lock().unwrap().clone()
                };

                let suggestion = {
                    let history_ref = history.borrow();
                    Self::build_autosuggestion(
                        &current_value,
                        cursor_pos,
                        &history_ref,
                        &current_path,
                    )
                };

                buffer::update_input_state(current_value, cursor_pos);
                buffer::update_autosuggestion(suggestion);
                buffer::auto_scroll_to_bottom();
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
            let history = history.clone();
            let processor = processor.clone();

            Closure::wrap(Box::new(move |event: KeyboardEvent| {
                if editor::is_active() {
                    event.prevent_default();
                    event.stop_propagation();
                    hidden_input.set_value("");

                    match editor::handle_key(&event) {
                        EditorEvent::Continue => {
                            editor::render(&terminal.renderer);
                        }
                        EditorEvent::Exit { message } => {
                            if let Some(message) = message {
                                if !message.trim().is_empty() {
                                    buffer::add_output_lines(&message, None);
                                }
                            }
                            Self::handle_input(&terminal, &hidden_input);
                        }
                    }
                    return;
                }

                let state = buffer::get_terminal_state();
                if state.input_mode == InputMode::Disabled {
                    event.prevent_default();
                    return;
                }
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
                        if let Some(cmd) = history.borrow_mut().prev(&current_input) {
                            hidden_input.set_value(&cmd);
                            CURRENT_INPUT.with(|input| {
                                *input.borrow_mut() = cmd.clone();
                            });

                            let cursor = cmd.chars().count();
                            let _ = hidden_input.set_selection_range(cursor as u32, cursor as u32);

                            buffer::update_autosuggestion(String::new());
                            buffer::update_input_state(cmd, cursor);
                            terminal.render();
                        }
                    }
                    "ArrowDown" => {
                        event.prevent_default();
                        if let Some(cmd) = history.borrow_mut().next() {
                            hidden_input.set_value(&cmd);
                            CURRENT_INPUT.with(|input| {
                                *input.borrow_mut() = cmd.clone();
                            });

                            let cursor = cmd.chars().count();
                            let _ = hidden_input.set_selection_range(cursor as u32, cursor as u32);

                            buffer::update_autosuggestion(String::new());
                            buffer::update_input_state(cmd, cursor);
                        } else {
                            hidden_input.set_value("");
                            CURRENT_INPUT.with(|input| {
                                input.borrow_mut().clear();
                            });
                            let _ = hidden_input.set_selection_range(0, 0);
                            buffer::update_autosuggestion(String::new());
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

                        let input_len = current_input.chars().count();

                        if current_cursor == input_len {
                            let state = buffer::get_terminal_state();
                            if !state.autosuggestion.is_empty()
                                && state.autosuggestion.starts_with(&current_input)
                                && state.autosuggestion != current_input
                            {
                                let suggested = state.autosuggestion;
                                hidden_input.set_value(&suggested);
                                CURRENT_INPUT.with(|input| {
                                    *input.borrow_mut() = suggested.clone();
                                });

                                let cursor = suggested.chars().count();
                                let _ =
                                    hidden_input.set_selection_range(cursor as u32, cursor as u32);

                                buffer::update_autosuggestion(String::new());
                                buffer::update_input_state(suggested, cursor);
                                buffer::auto_scroll_to_bottom();
                                terminal.render();
                                return;
                            }
                        }

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
                        Self::handle_tab(&terminal, &hidden_input, &current_input);
                    }
                    "PageUp" => {
                        event.prevent_default();
                        if buffer::scroll_up(10) {
                            terminal.render();
                        }
                    }
                    "PageDown" => {
                        event.prevent_default();
                        if buffer::scroll_down(10) {
                            terminal.render();
                        }
                    }
                    "l" | "L" if event.ctrl_key() => {
                        event.prevent_default();
                        buffer::clear_buffer();
                        buffer::reset_scroll();
                        buffer::update_autosuggestion(String::new());
                        terminal.render();
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
        let state = buffer::get_terminal_state();
        if state.input_mode == InputMode::Disabled {
            return;
        }
        let trimmed_input = current_input.trim();

        if panic::should_panic(trimmed_input) {
            Self::dispatch_command_event(trimmed_input);
            buffer::reset_scroll();
            history.add(trimmed_input.to_string());
            let prompt = terminal.get_current_prompt();
            buffer::add_command_line(&prompt, trimmed_input);

            hidden_input.set_value("");
            CURRENT_INPUT.with(|input| input.borrow_mut().clear());
            buffer::update_input_state(String::new(), 0);
            buffer::update_autosuggestion(String::new());
            buffer::set_input_mode(InputMode::Disabled);

            let terminal_clone = terminal.clone();
            let hidden_input_clone = hidden_input.clone();
            spawn_local(async move {
                panic::trigger(&terminal_clone).await;
                Self::handle_input(&terminal_clone, &hidden_input_clone);
            });
            return;
        }

        if !trimmed_input.is_empty() {
            Self::dispatch_command_event(trimmed_input);
            buffer::reset_scroll();
            history.add(trimmed_input.to_string());
            let prompt = terminal.get_current_prompt();
            buffer::add_command_line(&prompt, trimmed_input);
        }

        hidden_input.set_value("");
        CURRENT_INPUT.with(|input| input.borrow_mut().clear());
        buffer::update_input_state(String::new(), 0);
        buffer::update_autosuggestion(String::new());
        buffer::set_input_mode(InputMode::Disabled);

        if !trimmed_input.is_empty() {
            let (result, _directory_changed) = processor.handle(trimmed_input);

            match result {
                crate::commands::processor::CommandResult::Output(s) => match s.as_str() {
                    "CLEAR_SCREEN" => {
                        buffer::clear_buffer();
                        Self::handle_input(terminal, hidden_input);
                    }
                    "SYSTEM_PANIC" => {
                        let terminal_clone = terminal.clone();
                        let hidden_input_clone = hidden_input.clone();
                        spawn_local(async move {
                            panic::trigger(&terminal_clone).await;
                            Self::handle_input(&terminal_clone, &hidden_input_clone);
                        });
                    }
                    other if other.starts_with("__OPEN_EDITOR__:") => {
                        let target = other.trim_start_matches("__OPEN_EDITOR__:").trim();
                        match editor::open(target) {
                            Ok(()) => {
                                hidden_input.set_value("");
                                CURRENT_INPUT.with(|input| input.borrow_mut().clear());
                                editor::render(&terminal.renderer);
                            }
                            Err(error) => {
                                buffer::add_output_lines(&error, None);
                                Self::handle_input(terminal, hidden_input);
                            }
                        }
                    }
                    other if !other.is_empty() => {
                        buffer::add_output_lines(other, None);
                        Self::handle_input(terminal, hidden_input);
                    }
                    _ => {
                        Self::handle_input(terminal, hidden_input);
                    }
                },
                crate::commands::processor::CommandResult::Animated(animation) => {
                    let terminal_clone = terminal.clone();
                    let hidden_input_clone = hidden_input.clone();

                    spawn_local(async move {
                        animation(terminal_clone.renderer.clone()).await;
                        Self::handle_input(&terminal_clone, &hidden_input_clone);
                    });
                }
            }
        } else {
            Self::handle_input(terminal, hidden_input);
        }
    }

    fn handle_input(terminal: &Terminal, hidden_input: &HtmlInputElement) {
        let prompt = terminal.get_current_prompt();
        buffer::set_current_prompt(prompt);
        buffer::set_input_mode(InputMode::Normal);
        buffer::reset_scroll();
        buffer::update_autosuggestion(String::new());

        terminal.render();

        if let Some(window) = window() {
            if let Ok(event) = web_sys::CustomEvent::new("terminalFocus") {
                let _ = window.dispatch_event(&event);
            }
        }

        let _ = hidden_input.focus();
    }

    fn dispatch_command_event(command: &str) {
        if command.trim().is_empty() {
            return;
        }

        if let Some(window) = window() {
            if let Ok(event) = CustomEvent::new("terminalCommand") {
                event.init_custom_event_with_can_bubble_and_cancelable_and_detail(
                    "terminalCommand",
                    false,
                    false,
                    &JsValue::from_str(command),
                );
                let _ = window.dispatch_event(&event);
            }
        }
    }

    fn handle_tab(terminal: &Terminal, hidden_input: &HtmlInputElement, current_input: &str) {
        let state = buffer::get_terminal_state();
        if state.input_mode == InputMode::Disabled {
            return;
        }
        let current_path = {
            use crate::commands::filesystem::CURRENT_PATH;
            CURRENT_PATH.lock().unwrap().clone()
        };

        let cursor_pos = hidden_input
            .selection_start()
            .ok()
            .flatten()
            .map(|position| position as usize)
            .unwrap_or_else(|| current_input.len());

        let completion_result = AUTOCOMPLETE.with(|autocomplete| {
            autocomplete
                .borrow_mut()
                .complete(current_input, cursor_pos, &current_path)
        });

        match completion_result {
            CompletionResult::None => {}
            CompletionResult::Single(edit) => {
                Self::apply_completion_edit(terminal, hidden_input, edit);
            }
            CompletionResult::Multiple { options, common } => {
                if let Some(edit) = common {
                    Self::apply_completion_edit(terminal, hidden_input, edit);
                    return;
                }

                let completions_text = if options.len() <= 10 {
                    options.join("  ")
                } else {
                    let mut output = String::new();
                    for (i, completion) in options.iter().enumerate() {
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
                buffer::auto_scroll_to_bottom();

                let restored_cursor = cursor_pos.min(current_input.len());
                let _ = hidden_input
                    .set_selection_range(restored_cursor as u32, restored_cursor as u32);

                buffer::update_input_state(current_input.to_string(), restored_cursor);
                buffer::update_autosuggestion(String::new());
                terminal.render();
            }
        }
    }

    fn apply_completion_edit(
        terminal: &Terminal,
        hidden_input: &HtmlInputElement,
        edit: crate::terminal::autocomplete::CompletionEdit,
    ) {
        hidden_input.set_value(&edit.input);

        CURRENT_INPUT.with(|input| {
            *input.borrow_mut() = edit.input.clone();
        });

        let cursor = edit.cursor.min(edit.input.len());
        let _ = hidden_input.set_selection_range(cursor as u32, cursor as u32);

        buffer::update_input_state(edit.input, cursor);
        buffer::update_autosuggestion(String::new());
        terminal.render();
    }

    fn build_autosuggestion(
        current_input: &str,
        cursor_pos: usize,
        history: &CommandHistory,
        current_path: &[String],
    ) -> String {
        if current_input.trim().is_empty() || cursor_pos < current_input.chars().count() {
            return String::new();
        }

        if let Some(history_match) = history.suggest(current_input) {
            return history_match;
        }

        let completion = AUTOCOMPLETE.with(|autocomplete| {
            autocomplete
                .borrow_mut()
                .complete(current_input, current_input.len(), current_path)
        });

        match completion {
            CompletionResult::Single(edit) => {
                if edit.input.starts_with(current_input) {
                    edit.input
                } else {
                    String::new()
                }
            }
            CompletionResult::Multiple { common, .. } => {
                if let Some(edit) = common {
                    if edit.input.starts_with(current_input) {
                        edit.input
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                }
            }
            CompletionResult::None => {
                let mut commands = registry::command_names();
                commands.sort();

                commands
                    .into_iter()
                    .find(|candidate| {
                        candidate.starts_with(current_input) && candidate.as_str() != current_input
                    })
                    .unwrap_or_default()
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

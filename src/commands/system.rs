use crate::ascii::AsciiArt;
use crate::commands::{
    core,
    options::{self, OptionSpec},
};
use std::sync::OnceLock;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = Date)]
    fn now() -> f64;
}

static START_TIME: OnceLock<f64> = OnceLock::new();
pub fn init() {
    START_TIME.set(now()).ok();
}

pub fn clear(args: &[&str]) -> String {
    let options = match options::parse("clear", args, OptionSpec::new(&[], &["help"])) {
        Ok(options) => options,
        Err(error) => return error,
    };

    if options.has_help() {
        return "Usage: clear".to_string();
    }

    if let Err(error) = options::no_args("clear", &options.operands) {
        return error;
    }

    "CLEAR_SCREEN".to_string()
}

pub fn echo(args: &[&str]) -> String {
    let options = match options::parse("echo", args, OptionSpec::new(&['n'], &["help"])) {
        Ok(options) => options,
        Err(error) => return error,
    };

    if options.has_help() {
        return "Usage: echo [-n] [text ...]\n  -n   accepted for compatibility".to_string();
    }

    if options.operands.is_empty() {
        String::new()
    } else if options.operands.len() == 1 && options.operands[0] == "$USER" {
        AsciiArt::get_user()
    } else {
        let expanded: Vec<String> = options
            .operands
            .iter()
            .map(|arg| expand_variable(arg))
            .collect();
        expanded.join(" ")
    }
}

fn expand_variable(arg: &str) -> String {
    match arg {
        "$HOME" => "/home/objz".to_string(),
        "$PWD" => core::pwd(&[]),
        "$USER" => "objz".to_string(),
        "$SHELL" => "/bin/zsh".to_string(),
        "$HOSTNAME" => "portfolio".to_string(),
        _ => arg.to_string(),
    }
}

pub fn date(args: &[&str]) -> String {
    let options = match options::parse("date", args, OptionSpec::new(&[], &["help"])) {
        Ok(options) => options,
        Err(error) => return error,
    };

    if options.has_help() {
        return "Usage: date".to_string();
    }

    if let Err(error) = options::no_args("date", &options.operands) {
        return error;
    }

    let millis = now();
    let date = js_sys::Date::new(&JsValue::from_f64(millis));
    date.to_iso_string().into()
}

pub fn uptime(args: &[&str]) -> String {
    let options = match options::parse("uptime", args, OptionSpec::new(&[], &["help"])) {
        Ok(options) => options,
        Err(error) => return error,
    };

    if options.has_help() {
        return "Usage: uptime".to_string();
    }

    if let Err(error) = options::no_args("uptime", &options.operands) {
        return error;
    }

    let start = *START_TIME.get().unwrap_or(&now());
    let elapsed = now() - start;

    let total_secs = (elapsed / 1000.0) as u64;
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;

    format!("{:02}h {:02}m {:02}s", hours, minutes, seconds)
}

pub fn neofetch(args: &[&str]) -> String {
    let options = match options::parse("neofetch", args, OptionSpec::new(&[], &["help"])) {
        Ok(options) => options,
        Err(error) => return error,
    };

    if options.has_help() {
        return "Usage: neofetch".to_string();
    }

    if let Err(error) = options::no_args("neofetch", &options.operands) {
        return error;
    }

    let uptime_str = uptime(&[]);
    let resolution_str = get_resolution();

    format!(
        r#"
             .           
             7:          objz@portfolio
           .7J^          -----------------
         .~?JJ:          OS: WASM Linux x86_64
       :!?JJJ~           Host: GitHub Pages
     ^7JJJJ7:            Kernel: WASM 6.8.9
   :7JJJJ7:   .^         Uptime: {}
  :?J?J?:     :J7.       Packages: 23 (rust),
  ~J?J?.      !JJ!       Shell: objz-shell
  .7JJ!     .7J?J?       Resolution: {}
    :~7:  .~?J?J?^       WM: tty1
        .~?JJJ?!:        Theme: Dark
       :?JJJ?!:          Icons: ASCII Art Pack
      .?JJ7^.            Terminal: objz-term
      :JJ~               Memory: 521MiB / ∞ GiB
       7!                
                       
"#,
        uptime_str, resolution_str
    )
}

fn get_resolution() -> String {
    if let Some(window) = web_sys::window() {
        let width = window.inner_width().unwrap().as_f64().unwrap() as i32;
        let height = window.inner_height().unwrap().as_f64().unwrap() as i32;
        format!("{}x{}", width, height)
    } else {
        "1920x1080".to_string()
    }
}

pub fn hostname(args: &[&str]) -> String {
    let options = match options::parse("hostname", args, OptionSpec::new(&[], &["help"])) {
        Ok(options) => options,
        Err(error) => return error,
    };

    if options.has_help() {
        return "Usage: hostname".to_string();
    }

    if let Err(error) = options::no_args("hostname", &options.operands) {
        return error;
    }

    "portfolio".to_string()
}

pub fn whoami(args: &[&str]) -> String {
    let options = match options::parse("whoami", args, OptionSpec::new(&[], &["help"])) {
        Ok(options) => options,
        Err(error) => return error,
    };

    if options.has_help() {
        return "Usage: whoami".to_string();
    }

    if let Err(error) = options::no_args("whoami", &options.operands) {
        return error;
    }

    "objz".to_string()
}

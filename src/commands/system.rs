use crate::ascii::AsciiArt;
use std::sync::OnceLock;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = Date)]
    fn now() -> f64;

    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

static START_TIME: OnceLock<f64> = OnceLock::new();
pub fn init() {
    START_TIME.set(now()).ok();
}

pub fn clear(_args: &[&str]) -> String {
    "CLEAR_SCREEN".to_string()
}

pub fn echo(args: &[&str]) -> String {
    if args.is_empty() {
        String::new()
    } else if args[0] == "$USER" {
        AsciiArt::get_user()
    } else {
        args.join(" ")
    }
}

pub fn date(_args: &[&str]) -> String {
    let millis = now();
    let date = js_sys::Date::new(&JsValue::from_f64(millis));
    date.to_iso_string().into()
}

pub fn uptime(_args: &[&str]) -> String {
    let start = *START_TIME.get().unwrap_or(&now());
    let elapsed = now() - start;

    let total_secs = (elapsed / 1000.0) as u64;
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;

    format!("{:02}h {:02}m {:02}s", hours, minutes, seconds)
}

pub fn neofetch(_args: &[&str]) -> String {
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

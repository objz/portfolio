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
    r#"                   -`                    objz@portfolio
                  .o+`                   -----------------
                 `ooo/                   OS: WASM Linux x86_64
                `+oooo:                  Host: GitHub Pages
               `+oooooo:                 Kernel: WASM 6.8.9
               -+oooooo+:                Uptime: 17 days, 13 hours, 28 mins
             `/:-:++oooo+:               Packages: 42 (rust), 13 (npm)
            `/++++/+++++++:              Shell: objz-shell 3.0.0
           `/++++++++++++++:             Resolution: 1920x1080
          `/+++ooooooooo++++/            WM: tty1
         ./ooosssso++osssssso+`          Theme: Dark
        .oossssso-````/ossssss+`         Icons: ASCII Art Pack
       -osssssso.      :ssssssso.        Terminal: objz-term
      :osssssss/        +sssso+++.
     /ossssssss/        +ssssooo/-       Memory: 521MiB / ∞GiB
   `/ossssso+/:-        -:/+osssso+-     
  `+sso+:-`                 `.-/+oso:    
 `++:.                           `-/+/   
 .`                                 `/   "#
        .to_string()
}

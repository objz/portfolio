use crate::terminal::{renderer::LineOptions, Terminal};
use lazy_static::lazy_static;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BootConfig {
    boot_messages: Vec<String>,
    logo_lines: Vec<String>,
    login_messages: Vec<LoginMessage>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LoginMessage {
    text: String,
    color: Option<String>,
    typing_delay_ms: Option<u32>,
    masked_suffix: Option<String>,
    delay_ms: Option<u32>,
}

lazy_static! {
    static ref BOOT_CONFIG: BootConfig =
        serde_json::from_str(include_str!("../../static/content/boot.json"))
            .expect("static/content/boot.json must be valid");
}

pub async fn boot(term: &Terminal) {
    for message in &BOOT_CONFIG.boot_messages {
        term.add_line(message, Some(LineOptions::new().with_animation()))
            .await;
        term.sleep(15).await;
    }

    term.sleep(200).await;
}

pub async fn logo(term: &Terminal) {
    for line in &BOOT_CONFIG.logo_lines {
        term.add_line(line, Some(LineOptions::new().with_color("cyan")))
            .await;
        term.sleep(10).await;
    }
}

pub async fn login(term: &Terminal) {
    for entry in &BOOT_CONFIG.login_messages {
        if entry.text.is_empty() {
            term.add_line("", None).await;
            let delay = entry.delay_ms.unwrap_or(60) as i32;
            term.sleep(delay).await;
            continue;
        }

        let color = entry.color.as_deref().unwrap_or("white");

        if let Some(typing_delay) = entry.typing_delay_ms {
            let rendered = if let Some(masked_suffix) = &entry.masked_suffix {
                format!("{}{}", entry.text, masked_suffix)
            } else {
                entry.text.clone()
            };

            let typing_delay = typing_delay.min(i32::MAX as u32) as i32;

            term.add_line(
                &rendered,
                Some(LineOptions::new().with_typing(typing_delay)),
            )
            .await;
        } else {
            term.add_line(&entry.text, Some(LineOptions::new().with_color(color)))
                .await;
        }

        let delay = entry.delay_ms.unwrap_or(60) as i32;
        term.sleep(delay).await;
    }
}

use crate::commands::options::{self, OptionSpec};

pub fn help(_args: &[&str]) -> String {
    let mut commands = crate::commands::registry::command_names();
    commands.sort();

    let lines = commands
        .chunks(8)
        .map(|chunk| format!("  {}", chunk.join("  ")))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "Available commands:\n\n{}\n\nUse `<command> --help` for command-specific usage.",
        lines
    )
}

pub fn sudo(args: &[&str]) -> String {
    if args.is_empty() {
        return "sudo: command required".into();
    }

    let command = args[0];

    if crate::commands::registry::is_known_command(command) {
        "sudo: access denied.".into()
    } else {
        format!("zsh: command not found: {}", command)
    }
}

pub fn cowsay(args: &[&str]) -> String {
    if args.iter().any(|arg| *arg == "--help") {
        return "Usage: cowsay <message>\nExample: cowsay hello world".to_string();
    }

    let message = if args.is_empty() {
        "Moo"
    } else {
        &args.join(" ")
    };

    let lines = wrap_words(message, 40);
    let width = lines
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0);

    let mut bubble = String::new();
    bubble.push(' ');
    bubble.push_str(&"_".repeat(width + 2));
    bubble.push('\n');

    if lines.len() == 1 {
        bubble.push_str(&format!("< {:width$} >\n", lines[0], width = width));
    } else {
        for (index, line) in lines.iter().enumerate() {
            let (left, right) = if index == 0 {
                ('/', '\\')
            } else if index + 1 == lines.len() {
                ('\\', '/')
            } else {
                ('|', '|')
            };

            bubble.push_str(&format!(
                "{} {:width$} {}\n",
                left,
                line,
                right,
                width = width
            ));
        }
    }

    bubble.push(' ');
    bubble.push_str(&"-".repeat(width + 2));
    bubble.push('\n');
    bubble.push_str(
        r#"        \
         \   ^__^
          \  (oo)\_______
             (__)\       )\/\
                 ||----w |
                 ||     ||"#,
    );

    bubble
}

fn wrap_words(input: &str, max_width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();

    for raw_line in input.split('\n') {
        if raw_line.trim().is_empty() {
            if !current.is_empty() {
                lines.push(current.clone());
                current.clear();
            }
            lines.push(String::new());
            continue;
        }

        for word in raw_line.split_whitespace() {
            let projected = if current.is_empty() {
                word.chars().count()
            } else {
                current.chars().count() + 1 + word.chars().count()
            };

            if projected > max_width && !current.is_empty() {
                lines.push(current.clone());
                current.clear();
            }

            if !current.is_empty() {
                current.push(' ');
            }
            current.push_str(word);
        }

        if !current.is_empty() {
            lines.push(current.clone());
            current.clear();
        }
    }

    if lines.is_empty() {
        vec![String::new()]
    } else {
        lines
    }
}

pub fn lolcat(args: &[&str]) -> String {
    let options = match options::parse(
        "lolcat",
        args,
        OptionSpec::new(&['f'], &["force-color", "help"]),
    ) {
        Ok(options) => options,
        Err(error) => return error,
    };

    if options.has_help() {
        return "Usage: lolcat [text]\nExamples:\n  echo hello | lolcat\n  lolcat ".to_string();
    }

    let input = if options.operands.is_empty() {
        return "lolcat: missing input\nTry: echo hello | lolcat".to_string();
    } else if options.operands.len() == 1 {
        options.operands[0].clone()
    } else {
        options.operands.join(" ")
    };

    render_lolcat(&input)
}

fn render_lolcat(input: &str) -> String {
    let mut output = String::new();
    let mut hue = 0.0_f64;

    for ch in input.chars() {
        if ch == '\n' {
            output.push_str("\x1b[0m\n");
            continue;
        }

        let (r, g, b) = hsv_to_rgb(hue, 0.85, 1.0);
        output.push_str(&format!("\x1b[38;2;{};{};{}m{}", r, g, b, ch));

        hue += 6.5;
        if hue >= 360.0 {
            hue -= 360.0;
        }
    }

    output.push_str("\x1b[0m");
    output
}

fn hsv_to_rgb(h: f64, s: f64, v: f64) -> (u8, u8, u8) {
    let c = v * s;
    let x = c * (1.0 - (((h / 60.0) % 2.0) - 1.0).abs());
    let m = v - c;

    let (r1, g1, b1) = match h as i32 {
        0..=59 => (c, x, 0.0),
        60..=119 => (x, c, 0.0),
        120..=179 => (0.0, c, x),
        180..=239 => (0.0, x, c),
        240..=299 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };

    let to_u8 = |value: f64| ((value + m) * 255.0).round().clamp(0.0, 255.0) as u8;
    (to_u8(r1), to_u8(g1), to_u8(b1))
}

pub fn calc(args: &[&str]) -> String {
    if args.is_empty() {
        return "Usage: calc <expression>\nExamples:\n  calc 2 + 2\n  calc (5 + 3) * 2".to_string();
    }

    let expression = args.join(" ");
    match meval::eval_str(&expression) {
        Ok(result) if result.is_finite() => format!("{} = {}", expression, result),
        Ok(_) => format!(
            "Error: expression produced a non-finite value for '{}'",
            expression
        ),
        Err(error) => format!("Error: {}", error),
    }
}

#[cfg(test)]
mod tests {
    use super::calc;

    #[test]
    fn calc_supports_exponent_operator() {
        assert_eq!(calc(&["5^2"]), "5^2 = 25");
    }

    #[test]
    fn calc_reports_parse_error() {
        assert!(calc(&["5**2"]).starts_with("Error:"));
    }
}

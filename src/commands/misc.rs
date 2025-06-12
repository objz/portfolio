pub fn help(_args: &[&str]) -> String {
    r#"Available commands:

System Info:
  uname       - System information
  uptime      - System uptime
  neofetch    - Detailed system info
  date        - Current date and time

File System:
  ls, ll      - List directory contents
  cd          - Change directory
  pwd         - Print working directory
  cat         - Display file contents
  tree        - Display directory tree
  mkdir       - Create directory
  touch       - Create empty file
  rm          - Remove files/directories
  ln          - Create symbolic links

Utilities:
  clear       - Clear screen
  history     - Command history
  echo        - Display text
  cowsay      - ASCII cow with message
  sl          - Steam locomotive
  lolcat      - Rainbow text
  calc        - Calculator
  sudo        - Sudo access


Type `ls`, then `cd projects` and `ls` again.  
Run a project with `./project-name`.""#
        .to_string()
}

pub fn sudo(args: &[&str]) -> String {
    if args.is_empty() {
        return "sudo: command required".into();
    }
    "Sudo access denied.".into()
}

pub fn cowsay(args: &[&str]) -> String {
    let message = if args.is_empty() {
        "Hello from WASM!"
    } else {
        &args.join(" ")
    };

    let bubble_line = "-".repeat(message.len() + 2);

    format!(
        r#" {}
< {} >
 {}
        \   ^__^
         \  (oo)\_______
            (__)\       )\/\
                ||----w |
                ||     ||"#,
        bubble_line, message, bubble_line
    )
}

pub fn sl(_args: &[&str]) -> String {
    r#"                 (@@) (  ) (@)  ( )  @@    ()    @     O     @     O      @
            (   )
        (@@@@)
     (    )

   (@@@)
====        ________                ___________
_D _|  |_______/        \__I_I_____===__|_________|
 |(_)---  |   H\________/ |   |        =|___ ___|      _________________
 /     |  |   H  |  |     |   |         ||_| |_||     _|                \_____A
|      |  |   H  |__--------------------| [___] |   =|                        |
| ________|___H__/__|_____/[][]~\_______|       |   -|                        |
|/ |   |-----------I_____I [][] []  D   |=======|____|________________________|_
__/ =| o |=-~O=====O=====O=====O\ ____Y___________|__|__________________________|_
 |/-=|___|=    ||    ||    ||    |_____/~\___/          |_D__D__D_|  |_D__D__D_|
  \_/      \__/  \__/  \__/  \__/      \_/               \_/   \_/    \_/   \_/

You have new mail."#
        .to_string()
}

pub fn lolcat(args: &[&str]) -> String {
    if args.is_empty() {
        "Usage: lolcat <text>".to_string()
    } else {
        format!("🌈 {} 🌈", args.join(" "))
    }
}

pub fn calc(args: &[&str]) -> String {
    if args.is_empty() {
        return "Usage: calc <expression>\nExample: calc 2 + 2".to_string();
    }

    let expression = args.join(" ");

    if let Some(result) = evaluate(&expression) {
        format!("{} = {}", expression, result)
    } else {
        format!("Error: Cannot evaluate '{}'", expression)
    }
}

fn evaluate(expr: &str) -> Option<f64> {
    let parts: Vec<&str> = expr.split_whitespace().collect();

    if parts.len() == 3 {
        if let (Ok(a), Ok(b)) = (parts[0].parse::<f64>(), parts[2].parse::<f64>()) {
            match parts[1] {
                "+" => Some(a + b),
                "-" => Some(a - b),
                "*" => Some(a * b),
                "/" if b != 0.0 => Some(a / b),
                _ => None,
            }
        } else {
            None
        }
    } else {
        None
    }
}

use std::collections::HashSet;

#[derive(Clone, Debug, Default)]
pub struct ParsedOptions {
    short_flags: HashSet<char>,
    long_flags: HashSet<String>,
    pub operands: Vec<String>,
}

impl ParsedOptions {
    pub fn has_short(&self, flag: char) -> bool {
        self.short_flags.contains(&flag)
    }

    pub fn has_long(&self, flag: &str) -> bool {
        self.long_flags.contains(flag)
    }

    pub fn has_help(&self) -> bool {
        self.has_long("help")
    }
}

#[derive(Clone, Copy, Debug)]
pub struct OptionSpec {
    pub short_flags: &'static [char],
    pub long_flags: &'static [&'static str],
}

impl OptionSpec {
    pub const fn new(short_flags: &'static [char], long_flags: &'static [&'static str]) -> Self {
        Self {
            short_flags,
            long_flags,
        }
    }
}

pub fn parse(command: &str, args: &[&str], spec: OptionSpec) -> Result<ParsedOptions, String> {
    let mut parsed = ParsedOptions::default();
    let mut parse_options = true;

    for arg in args {
        if !parse_options {
            parsed.operands.push((*arg).to_string());
            continue;
        }

        if *arg == "--" {
            parse_options = false;
            continue;
        }

        if let Some(long_name) = arg.strip_prefix("--") {
            if long_name.is_empty() {
                parse_options = false;
                continue;
            }

            if spec.long_flags.contains(&long_name) {
                parsed.long_flags.insert(long_name.to_string());
            } else {
                return Err(format!(
                    "{}: unrecognized option '{}'; try --help",
                    command, arg
                ));
            }
            continue;
        }

        if arg.starts_with('-') && arg.len() > 1 {
            for flag in arg.chars().skip(1) {
                if spec.short_flags.contains(&flag) {
                    parsed.short_flags.insert(flag);
                } else {
                    return Err(format!(
                        "{}: invalid option -- '{}'; try --help",
                        command, flag
                    ));
                }
            }
            continue;
        }

        parsed.operands.push((*arg).to_string());
    }

    Ok(parsed)
}

pub fn no_args(command: &str, args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        Ok(())
    } else {
        Err(format!("{}: too many arguments", command))
    }
}

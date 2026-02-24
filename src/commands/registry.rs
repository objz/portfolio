#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PathCompletionMode {
    DirectoriesOnly,
    FilesAndDirectories,
    ProjectRepos,
}

#[derive(Clone, Copy, Debug)]
pub struct CommandSpec {
    pub name: &'static str,
    pub aliases: &'static [&'static str],
    pub path_completion: Option<PathCompletionMode>,
}

pub const COMMANDS: &[CommandSpec] = &[
    CommandSpec {
        name: "clear",
        aliases: &[],
        path_completion: None,
    },
    CommandSpec {
        name: "history",
        aliases: &[],
        path_completion: None,
    },
    CommandSpec {
        name: "echo",
        aliases: &[],
        path_completion: None,
    },
    CommandSpec {
        name: "date",
        aliases: &[],
        path_completion: None,
    },
    CommandSpec {
        name: "uptime",
        aliases: &[],
        path_completion: None,
    },
    CommandSpec {
        name: "neofetch",
        aliases: &[],
        path_completion: None,
    },
    CommandSpec {
        name: "uname",
        aliases: &[],
        path_completion: None,
    },
    CommandSpec {
        name: "hostname",
        aliases: &[],
        path_completion: None,
    },
    CommandSpec {
        name: "whoami",
        aliases: &[],
        path_completion: None,
    },
    CommandSpec {
        name: "ls",
        aliases: &["ll"],
        path_completion: Some(PathCompletionMode::FilesAndDirectories),
    },
    CommandSpec {
        name: "cd",
        aliases: &[],
        path_completion: Some(PathCompletionMode::DirectoriesOnly),
    },
    CommandSpec {
        name: "cat",
        aliases: &[],
        path_completion: Some(PathCompletionMode::FilesAndDirectories),
    },
    CommandSpec {
        name: "pwd",
        aliases: &[],
        path_completion: None,
    },
    CommandSpec {
        name: "tree",
        aliases: &[],
        path_completion: Some(PathCompletionMode::FilesAndDirectories),
    },
    CommandSpec {
        name: "mkdir",
        aliases: &[],
        path_completion: Some(PathCompletionMode::FilesAndDirectories),
    },
    CommandSpec {
        name: "touch",
        aliases: &[],
        path_completion: Some(PathCompletionMode::FilesAndDirectories),
    },
    CommandSpec {
        name: "rm",
        aliases: &[],
        path_completion: Some(PathCompletionMode::FilesAndDirectories),
    },
    CommandSpec {
        name: "ln",
        aliases: &[],
        path_completion: Some(PathCompletionMode::FilesAndDirectories),
    },
    CommandSpec {
        name: "cp",
        aliases: &[],
        path_completion: Some(PathCompletionMode::FilesAndDirectories),
    },
    CommandSpec {
        name: "mv",
        aliases: &[],
        path_completion: Some(PathCompletionMode::FilesAndDirectories),
    },
    CommandSpec {
        name: "nvim",
        aliases: &[],
        path_completion: Some(PathCompletionMode::FilesAndDirectories),
    },
    CommandSpec {
        name: "help",
        aliases: &[],
        path_completion: None,
    },
    CommandSpec {
        name: "sudo",
        aliases: &[],
        path_completion: None,
    },
    CommandSpec {
        name: "cowsay",
        aliases: &[],
        path_completion: None,
    },
    CommandSpec {
        name: "sl",
        aliases: &[],
        path_completion: None,
    },
    CommandSpec {
        name: "lolcat",
        aliases: &[],
        path_completion: None,
    },
    CommandSpec {
        name: "calc",
        aliases: &[],
        path_completion: None,
    },
    CommandSpec {
        name: "project",
        aliases: &[],
        path_completion: Some(PathCompletionMode::ProjectRepos),
    },
    CommandSpec {
        name: "readme",
        aliases: &[],
        path_completion: Some(PathCompletionMode::ProjectRepos),
    },
    CommandSpec {
        name: "open",
        aliases: &[],
        path_completion: Some(PathCompletionMode::ProjectRepos),
    },
];

pub fn command_names() -> Vec<String> {
    let mut names = Vec::new();

    for spec in COMMANDS {
        names.push(spec.name.to_string());
        names.extend(spec.aliases.iter().map(|alias| alias.to_string()));
    }

    names.sort();
    names.dedup();
    names
}

pub fn is_known_command(command: &str) -> bool {
    COMMANDS
        .iter()
        .any(|spec| spec.name == command || spec.aliases.contains(&command))
}

pub fn path_completion_mode(command: &str) -> Option<PathCompletionMode> {
    COMMANDS
        .iter()
        .find(|spec| spec.name == command || spec.aliases.contains(&command))
        .and_then(|spec| spec.path_completion)
}

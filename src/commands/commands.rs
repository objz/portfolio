use crate::commands::filesystem::{
    get_node, get_node_mut, normalize_path, project_sort_rank, Node, CURRENT_PATH, CURRENT_USER,
    FILESYSTEM,
};
use crate::commands::options::{self, OptionSpec};
use std::collections::HashMap;

pub fn ls(args: &[&str]) -> String {
    let options = match options::parse(
        "ls",
        args,
        OptionSpec::new(&['a', 'l', '1'], &["all", "long", "oneline", "help"]),
    ) {
        Ok(options) => options,
        Err(error) => return error,
    };

    if options.has_help() {
        return "Usage: ls [-a] [-l] [-1] [path]\n  -a, --all      show hidden entries\n  -l, --long     use long listing format\n  -1, --oneline  one entry per line"
            .to_string();
    }

    if options.operands.len() > 1 {
        return "ls: too many arguments".to_string();
    }

    let show_hidden = options.has_short('a') || options.has_long("all");
    let long_format = options.has_short('l') || options.has_long("long");
    let one_per_line = options.has_short('1') || options.has_long("oneline") || long_format;

    let filesystem = FILESYSTEM.lock().unwrap();
    let current_path = CURRENT_PATH.lock().unwrap();

    let target_path = options.operands.first().map(String::as_str);

    let path = if let Some(target) = target_path {
        normalize_path(target, &current_path)
    } else {
        current_path.clone()
    };

    let node = match get_node(&filesystem, &path) {
        Some(node) => node,
        None => return "ls: cannot access: No such file or directory".into(),
    };

    match node {
        Node::Directory { children, .. } => {
            let mut entries: Vec<_> = children
                .iter()
                .filter(|(name, _)| show_hidden || !name.starts_with('.'))
                .collect();

            entries.sort_by(|(left_name, left_node), (right_name, right_node)| {
                let left_rank = project_sort_rank(&path, left_name);
                let right_rank = project_sort_rank(&path, right_name);

                match (left_rank, right_rank) {
                    (Some(a), Some(b)) => a.cmp(&b),
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => {
                        let kind_cmp = entry_kind(left_node);
                        let kind_cmp_right = entry_kind(right_node);

                        kind_cmp
                            .cmp(&kind_cmp_right)
                            .then_with(|| left_name.to_lowercase().cmp(&right_name.to_lowercase()))
                    }
                }
            });

            if long_format {
                entries
                    .iter()
                    .map(|(name, node)| {
                        let perm = permission_string(node);
                        let owner = node.get_owner();
                        let group = "objz";
                        let size = human_size(entry_size(node));
                        let display = style_entry_name(name, node);

                        format!(
                            "{} {:>4} {:<8} {:<8} {:>6} {} {}",
                            perm, 1, owner, group, size, "Jan 01 12:00", display
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            } else {
                let styled: Vec<(String, usize)> = entries
                    .iter()
                    .map(|(name, node)| {
                        let display_name = plain_entry_name(name, node);
                        (style_entry_name(name, node), display_name.chars().count())
                    })
                    .collect();

                if styled.is_empty() {
                    return String::new();
                }

                if one_per_line {
                    styled
                        .into_iter()
                        .map(|(name, _)| name)
                        .collect::<Vec<_>>()
                        .join("\n")
                } else {
                    let max_plain_width = styled.iter().map(|(_, width)| *width).max().unwrap_or(1);
                    let cell_width = max_plain_width + 2;
                    let columns = (84usize / cell_width.max(1)).max(1);

                    let mut lines = Vec::new();
                    for row in styled.chunks(columns) {
                        let mut line = String::new();
                        for (index, (name, width)) in row.iter().enumerate() {
                            if index > 0 {
                                line.push_str("  ");
                            }

                            line.push_str(name);
                            let padding = max_plain_width.saturating_sub(*width);
                            if padding > 0 {
                                line.push_str(&" ".repeat(padding));
                            }
                        }
                        lines.push(line);
                    }

                    lines.join("\n")
                }
            }
        }
        Node::File { .. } => target_path.unwrap_or(".").to_string(),
        Node::Symlink { target, .. } => format!("-> {}", target),
    }
}

fn entry_kind(node: &Node) -> usize {
    match node {
        Node::Directory { .. } => 0,
        Node::Symlink { .. } => 1,
        Node::File { .. } => 2,
    }
}

fn plain_entry_name(name: &str, node: &Node) -> String {
    match node {
        Node::Directory { .. } => format!("{}/", name),
        Node::Symlink { .. } => format!("{}@", name),
        Node::File { .. } => name.to_string(),
    }
}

fn style_entry_name(name: &str, node: &Node) -> String {
    match node {
        Node::Directory { .. } => format!("\x1b[1;34m{}/\x1b[0m", name),
        Node::Symlink { .. } => format!("\x1b[1;36m{}@\x1b[0m", name),
        Node::File { permissions, .. } => {
            if *permissions & 0o111 != 0 {
                format!("\x1b[1;32m{}\x1b[0m", name)
            } else if name.ends_with(".md") || name.ends_with(".txt") {
                format!("\x1b[0;37m{}\x1b[0m", name)
            } else {
                name.to_string()
            }
        }
    }
}

fn entry_size(node: &Node) -> usize {
    match node {
        Node::Directory { .. } => 4096,
        Node::Symlink { target, .. } => target.len(),
        Node::File { content, .. } => content.len(),
    }
}

fn human_size(size: usize) -> String {
    const UNITS: [&str; 5] = ["B", "K", "M", "G", "T"];

    let mut value = size as f64;
    let mut unit = 0usize;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }

    if unit == 0 {
        format!("{}{}", size, UNITS[unit])
    } else {
        format!("{:.1}{}", value, UNITS[unit])
    }
}

fn permission_string(node: &Node) -> String {
    let (kind, mode) = match node {
        Node::Directory { permissions, .. } => ('d', *permissions),
        Node::File { permissions, .. } => ('-', *permissions),
        Node::Symlink { .. } => ('l', 0o777),
    };

    let mut out = String::with_capacity(10);
    out.push(kind);

    for shift in [6u16, 3u16, 0u16] {
        let bits = (mode >> shift) & 0b111;
        out.push(if bits & 0b100 != 0 { 'r' } else { '-' });
        out.push(if bits & 0b010 != 0 { 'w' } else { '-' });
        out.push(if bits & 0b001 != 0 { 'x' } else { '-' });
    }

    out
}

pub fn cd(args: &[&str]) -> String {
    let options = match options::parse("cd", args, OptionSpec::new(&[], &["help"])) {
        Ok(options) => options,
        Err(error) => return error,
    };

    if options.has_help() {
        return "Usage: cd [directory]".to_string();
    }

    if options.operands.len() > 1 {
        return "cd: too many arguments".to_string();
    }

    if options.operands.is_empty() {
        {
            let mut path = CURRENT_PATH.lock().unwrap();
            *path = vec!["home".to_string(), "objz".to_string()];
        }
        return String::new();
    }

    let target = options.operands[0].as_str();

    let new_path = {
        let filesystem = FILESYSTEM.lock().unwrap();
        let current_path = CURRENT_PATH.lock().unwrap();

        let new_path = normalize_path(target, &current_path);

        match get_node(&filesystem, &new_path) {
            Some(Node::Directory { .. }) => new_path,
            Some(Node::Symlink { target, .. }) => {
                let symlink_path = normalize_path(target, &current_path);
                match get_node(&filesystem, &symlink_path) {
                    Some(Node::Directory { .. }) => symlink_path,
                    Some(_) => return format!("cd: {}: Not a directory", target),
                    None => return format!("cd: {}: No such file or directory", target),
                }
            }
            Some(_) => return format!("cd: {}: Not a directory", target),
            None => return format!("cd: {}: No such file or directory", target),
        }
    };

    {
        let mut path = CURRENT_PATH.lock().unwrap();
        *path = new_path;
    }
    String::new()
}

pub fn pwd(args: &[&str]) -> String {
    let options = match options::parse(
        "pwd",
        args,
        OptionSpec::new(&['L', 'P'], &["help", "logical", "physical"]),
    ) {
        Ok(options) => options,
        Err(error) => return error,
    };

    if options.has_help() {
        return "Usage: pwd [-L|-P]".to_string();
    }

    if let Err(error) = options::no_args("pwd", &options.operands) {
        return error;
    }

    let path = CURRENT_PATH.lock().unwrap();
    if path.is_empty() {
        "/".into()
    } else {
        format!("/{}", path.join("/"))
    }
}

pub fn cat(args: &[&str]) -> String {
    let options = match options::parse("cat", args, OptionSpec::new(&['n'], &["help", "number"])) {
        Ok(options) => options,
        Err(error) => return error,
    };

    if options.has_help() {
        return "Usage: cat [-n] [--number] <file>...".to_string();
    }

    if options.operands.is_empty() {
        return "cat: missing file operand".into();
    }

    let number_lines = options.has_short('n') || options.has_long("number");

    let filesystem = FILESYSTEM.lock().unwrap();
    let current_path = CURRENT_PATH.lock().unwrap();

    let mut output = String::new();
    let mut line_number = 1usize;

    for filename in &options.operands {
        let file_path = normalize_path(filename, &current_path);

        let chunk = match get_node(&filesystem, &file_path) {
            Some(Node::File { content, .. }) => {
                if number_lines {
                    format_numbered_content(content, &mut line_number)
                } else {
                    content.to_string()
                }
            }
            Some(Node::Directory { .. }) => format!("cat: {}: Is a directory", filename),
            Some(Node::Symlink { target, .. }) => {
                let target_path = normalize_path(target, &current_path);
                match get_node(&filesystem, &target_path) {
                    Some(Node::File { content, .. }) => {
                        if number_lines {
                            format_numbered_content(content, &mut line_number)
                        } else {
                            content.to_string()
                        }
                    }
                    Some(Node::Directory { .. }) => format!("cat: {}: Is a directory", filename),
                    Some(Node::Symlink { .. }) => {
                        format!("cat: {}: Too many levels of symbolic links", filename)
                    }
                    None => format!("cat: {}: No such file or directory", filename),
                }
            }
            None => format!("cat: {}: No such file or directory", filename),
        };

        if !chunk.is_empty() {
            if !output.is_empty() {
                output.push('\n');
            }
            output.push_str(chunk.trim_end_matches('\n'));
        }
    }

    output
}

fn format_numbered_content(content: &str, next_line_number: &mut usize) -> String {
    let mut numbered = String::new();

    for line in content.lines() {
        numbered.push_str(&format!("{:>6}\t{}\n", *next_line_number, line));
        *next_line_number += 1;
    }

    numbered.trim_end_matches('\n').to_string()
}

pub fn mkdir(args: &[&str]) -> String {
    let options = match options::parse("mkdir", args, OptionSpec::new(&['p'], &["parents", "help"]))
    {
        Ok(options) => options,
        Err(error) => return error,
    };

    if options.has_help() {
        return "Usage: mkdir [-p] <directory>...\n  -p, --parents   create parent directories as needed"
            .to_string();
    }

    if options.operands.is_empty() {
        return "mkdir: missing operand".into();
    }

    let create_parents = options.has_short('p') || options.has_long("parents");

    let mut filesystem = FILESYSTEM.lock().unwrap();
    let current_path = CURRENT_PATH.lock().unwrap();
    let current_user = &*CURRENT_USER;

    for dirname in &options.operands {
        let dir_path = normalize_path(dirname, &current_path);

        if dir_path.is_empty() {
            if create_parents {
                continue;
            }
            return "mkdir: cannot create directory '/': File exists".into();
        }

        if create_parents {
            for depth in 0..dir_path.len() {
                let parent_path = &dir_path[..depth];
                let part = &dir_path[depth];

                let parent = match get_node_mut(&mut filesystem, parent_path) {
                    Some(Node::Directory { children, .. }) => children,
                    Some(_) => {
                        return format!(
                            "mkdir: cannot create directory '{}': Not a directory",
                            dirname
                        )
                    }
                    None => {
                        return format!(
                            "mkdir: cannot create directory '{}': No such file or directory",
                            dirname
                        )
                    }
                };

                if let Some(existing) = parent.get(part) {
                    if !matches!(existing, Node::Directory { .. }) {
                        return format!(
                            "mkdir: cannot create directory '{}': File exists",
                            dirname
                        );
                    }
                    continue;
                }

                parent.insert(
                    part.clone(),
                    Node::Directory {
                        permissions: 0o755,
                        owner: current_user.clone(),
                        protected: false,
                        children: HashMap::new(),
                    },
                );
            }
            continue;
        }

        let parent_path = &dir_path[..dir_path.len() - 1];
        let dir_name = &dir_path[dir_path.len() - 1];

        let parent = match get_node_mut(&mut filesystem, parent_path) {
            Some(Node::Directory { children, .. }) => children,
            Some(_) => {
                return format!(
                    "mkdir: cannot create directory '{}': Not a directory",
                    dirname
                )
            }
            None => {
                return format!(
                    "mkdir: cannot create directory '{}': No such file or directory",
                    dirname
                )
            }
        };

        if parent.contains_key(dir_name) {
            return format!("mkdir: cannot create directory '{}': File exists", dirname);
        }

        parent.insert(
            dir_name.clone(),
            Node::Directory {
                permissions: 0o755,
                owner: current_user.clone(),
                protected: false,
                children: HashMap::new(),
            },
        );
    }

    String::new()
}

pub fn touch(args: &[&str]) -> String {
    let options = match options::parse(
        "touch",
        args,
        OptionSpec::new(&['c'], &["no-create", "help"]),
    ) {
        Ok(options) => options,
        Err(error) => return error,
    };

    if options.has_help() {
        return "Usage: touch [-c] <file>...\n  -c, --no-create   do not create missing files"
            .to_string();
    }

    if options.operands.is_empty() {
        return "touch: missing file operand".into();
    }

    let no_create = options.has_short('c') || options.has_long("no-create");

    let mut filesystem = FILESYSTEM.lock().unwrap();
    let current_path = CURRENT_PATH.lock().unwrap();
    let current_user = &*CURRENT_USER;

    for filename in &options.operands {
        let file_path = normalize_path(filename, &current_path);

        if file_path.is_empty() {
            continue;
        }

        let parent_path = &file_path[..file_path.len() - 1];
        let file_name = &file_path[file_path.len() - 1];

        let parent = match get_node_mut(&mut filesystem, parent_path) {
            Some(Node::Directory { children, .. }) => children,
            Some(_) => return format!("touch: cannot touch '{}': Not a directory", filename),
            None => {
                return format!(
                    "touch: cannot touch '{}': No such file or directory",
                    filename
                )
            }
        };

        if !parent.contains_key(file_name) {
            if no_create {
                continue;
            }

            parent.insert(
                file_name.clone(),
                Node::File {
                    content: String::new(),
                    permissions: 0o644,
                    owner: current_user.clone(),
                    protected: false,
                },
            );
        }
    }

    String::new()
}

pub fn write_output(target: &str, content: &str, append: bool) -> Result<(), String> {
    let mut filesystem = FILESYSTEM.lock().unwrap();
    let current_path = CURRENT_PATH.lock().unwrap();
    let current_user = &*CURRENT_USER;

    let file_path = normalize_path(target, &current_path);

    if file_path.is_empty() {
        return Err("redir: cannot write to '/': Is a directory".to_string());
    }

    let parent_path = &file_path[..file_path.len() - 1];
    let file_name = &file_path[file_path.len() - 1];

    let parent = match get_node_mut(&mut filesystem, parent_path) {
        Some(Node::Directory { children, .. }) => children,
        Some(_) => return Err(format!("redir: cannot write '{}': Not a directory", target)),
        None => {
            return Err(format!(
                "redir: cannot write '{}': No such file or directory",
                target
            ))
        }
    };

    if let Some(node) = parent.get_mut(file_name) {
        match node {
            Node::File {
                content: existing,
                owner,
                protected,
                ..
            } => {
                if *protected {
                    return Err(format!(
                        "redir: cannot write '{}': Operation not permitted",
                        target
                    ));
                }

                if owner.as_str() != current_user && current_user != "root" {
                    return Err(format!(
                        "redir: cannot write '{}': Permission denied",
                        target
                    ));
                }

                if append {
                    if !existing.is_empty() && !content.is_empty() {
                        existing.push('\n');
                    }
                    existing.push_str(content);
                } else {
                    *existing = content.to_string();
                }

                Ok(())
            }
            Node::Directory { .. } => {
                Err(format!("redir: cannot write '{}': Is a directory", target))
            }
            Node::Symlink { .. } => Err(format!(
                "redir: cannot write '{}': Symlink targets not supported",
                target
            )),
        }
    } else {
        parent.insert(
            file_name.clone(),
            Node::File {
                content: content.to_string(),
                permissions: 0o644,
                owner: current_user.clone(),
                protected: false,
            },
        );

        Ok(())
    }
}

pub fn rm(args: &[&str]) -> String {
    let options = match options::parse(
        "rm",
        args,
        OptionSpec::new(&['r', 'R', 'f'], &["recursive", "force", "help"]),
    ) {
        Ok(options) => options,
        Err(error) => return error,
    };

    if options.has_help() {
        return "Usage: rm [-f] [-r] <file>...\n  -f, --force      ignore missing files and never prompt\n  -r, -R, --recursive  remove directories recursively"
            .to_string();
    }

    let recursive =
        options.has_short('r') || options.has_short('R') || options.has_long("recursive");
    let force = options.has_short('f') || options.has_long("force");

    if options.operands.is_empty() {
        if force {
            return String::new();
        }
        return "rm: missing operand".into();
    }

    let mut filesystem = FILESYSTEM.lock().unwrap();
    let current_path = CURRENT_PATH.lock().unwrap();
    let current_user = &*CURRENT_USER;

    for filename in &options.operands {
        let file_path = normalize_path(filename, &current_path);

        if file_path.is_empty() {
            if !force {
                return "rm: cannot remove '/': Permission denied".into();
            }
            continue;
        }

        let parent_path = &file_path[..file_path.len() - 1];
        let file_name = &file_path[file_path.len() - 1];

        let parent = match get_node_mut(&mut filesystem, parent_path) {
            Some(Node::Directory { children, .. }) => children,
            Some(_) => {
                if !force {
                    return format!("rm: cannot remove '{}': Not a directory", filename);
                }
                continue;
            }
            None => {
                if !force {
                    return format!(
                        "rm: cannot remove '{}': No such file or directory",
                        filename
                    );
                }
                continue;
            }
        };

        match parent.get(file_name) {
            Some(node) => {
                if node.is_protected() {
                    return format!(
                        "rm: cannot remove '{}': Operation not permitted (protected system file)",
                        filename
                    );
                }

                if node.get_owner() != current_user && current_user != "root" {
                    return format!(
                        "rm: cannot remove '{}': Permission denied (not owner)",
                        filename
                    );
                }

                match node {
                    Node::Directory { .. } => {
                        if !recursive {
                            if !force {
                                return format!("rm: cannot remove '{}': Is a directory", filename);
                            }
                            continue;
                        }
                        parent.remove(file_name);
                    }
                    Node::File { .. } | Node::Symlink { .. } => {
                        parent.remove(file_name);
                    }
                }
            }
            None => {
                if !force {
                    return format!(
                        "rm: cannot remove '{}': No such file or directory",
                        filename
                    );
                }
            }
        }
    }

    String::new()
}

pub fn tree(args: &[&str]) -> String {
    let options = match options::parse("tree", args, OptionSpec::new(&['a'], &["all", "help"])) {
        Ok(options) => options,
        Err(error) => return error,
    };

    if options.has_help() {
        return "Usage: tree [-a] [path]\n  -a, --all   include hidden entries".to_string();
    }

    if options.operands.len() > 1 {
        return "tree: too many arguments".to_string();
    }

    let show_hidden = options.has_short('a') || options.has_long("all");

    let filesystem = FILESYSTEM.lock().unwrap();
    let current_path = CURRENT_PATH.lock().unwrap();

    let start_path = if options.operands.is_empty() {
        current_path.clone()
    } else {
        normalize_path(&options.operands[0], &current_path)
    };

    let start_node = match get_node(&filesystem, &start_path) {
        Some(node) => node,
        None => return "tree: No such file or directory".into(),
    };

    fn build_tree(node: &Node, prefix: &str, show_hidden: bool) -> String {
        let mut output = String::new();

        if let Node::Directory { children, .. } = node {
            let mut entries: Vec<_> = children.iter().collect();
            entries.sort_by_key(|(name, _)| name.as_str());
            entries.retain(|(name, _)| show_hidden || !name.starts_with('.'));

            for (i, (name, child)) in entries.iter().enumerate() {
                let is_last_child = i == entries.len() - 1;
                let connector = if is_last_child {
                    "└── "
                } else {
                    "├── "
                };

                let display_name = match child {
                    Node::Directory { .. } => format!("{}/", name),
                    Node::File { .. } => name.to_string(),
                    Node::Symlink { target, .. } => format!("{} -> {}", name, target),
                };

                output.push_str(&format!("{}{}{}\n", prefix, connector, display_name));

                let new_prefix =
                    format!("{}{}", prefix, if is_last_child { "    " } else { "│   " });

                output.push_str(&build_tree(child, &new_prefix, show_hidden));
            }
        }

        output
    }

    let tree_name = if start_path.is_empty() {
        "/".to_string()
    } else {
        start_path.last().unwrap_or(&"/".to_string()).clone()
    };

    format!("{}\n{}", tree_name, build_tree(start_node, "", show_hidden))
}

pub fn ln(args: &[&str]) -> String {
    let options = match options::parse("ln", args, OptionSpec::new(&['s'], &["symbolic", "help"])) {
        Ok(options) => options,
        Err(error) => return error,
    };

    if options.has_help() {
        return "Usage: ln -s <target> <link_name>\n  -s, --symbolic   create symbolic links"
            .to_string();
    }

    let symbolic = options.has_short('s') || options.has_long("symbolic");
    if !symbolic {
        return "ln: hard links are not supported; use -s for symlinks".into();
    }

    if options.operands.len() < 2 {
        return "ln: missing destination file operand after target".into();
    }

    if options.operands.len() > 2 {
        return "ln: too many arguments".into();
    }

    let mut filesystem = FILESYSTEM.lock().unwrap();
    let current_path = CURRENT_PATH.lock().unwrap();
    let current_user = &*CURRENT_USER;

    let target = options.operands[0].as_str();
    let link_name = options.operands[1].as_str();

    let link_path = normalize_path(link_name, &current_path);

    if link_path.is_empty() {
        return "ln: cannot create link '/': File exists".into();
    }

    let parent_path = &link_path[..link_path.len() - 1];
    let file_name = &link_path[link_path.len() - 1];

    let parent = match get_node_mut(&mut filesystem, parent_path) {
        Some(Node::Directory { children, .. }) => children,
        Some(_) => return format!("ln: cannot create link '{}': Not a directory", link_name),
        None => {
            return format!(
                "ln: cannot create link '{}': No such file or directory",
                link_name
            )
        }
    };

    if parent.contains_key(file_name) {
        return format!("ln: cannot create link '{}': File exists", link_name);
    }

    parent.insert(
        file_name.clone(),
        Node::Symlink {
            target: target.to_string(),
            owner: current_user.clone(),
        },
    );

    String::new()
}

pub fn cp(args: &[&str]) -> String {
    let options = match options::parse(
        "cp",
        args,
        OptionSpec::new(&['r', 'R'], &["recursive", "help"]),
    ) {
        Ok(options) => options,
        Err(error) => return error,
    };

    if options.has_help() {
        return "Usage: cp [-r] <source> <destination>\n  -r, -R, --recursive   copy directories recursively"
            .to_string();
    }

    if options.operands.len() < 2 {
        return "cp: missing destination file operand".into();
    }

    if options.operands.len() > 2 {
        return "cp: too many arguments".into();
    }

    let recursive =
        options.has_short('r') || options.has_short('R') || options.has_long("recursive");

    let mut filesystem = FILESYSTEM.lock().unwrap();
    let current_path = CURRENT_PATH.lock().unwrap();
    let current_user = &*CURRENT_USER;

    let source = options.operands[0].as_str();
    let dest = options.operands[1].as_str();

    let source_path = normalize_path(source, &current_path);
    let dest_path = normalize_path(dest, &current_path);

    if source_path.is_empty() {
        return "cp: cannot copy '/': Permission denied".into();
    }

    // Get the source node and clone it
    let source_node = match get_node(&filesystem, &source_path) {
        Some(node) => node.clone(),
        None => return format!("cp: cannot stat '{}': No such file or directory", source),
    };

    // Check if source is a directory and recursive flag
    if matches!(source_node, Node::Directory { .. }) && !recursive {
        return format!("cp: -r not specified; omitting directory '{}'", source);
    }

    if dest_path.is_empty() {
        return "cp: cannot copy to '/': Permission denied".into();
    }

    let parent_path = &dest_path[..dest_path.len() - 1];
    let dest_name = &dest_path[dest_path.len() - 1];

    let parent = match get_node_mut(&mut filesystem, parent_path) {
        Some(Node::Directory { children, .. }) => children,
        Some(_) => return format!("cp: cannot create '{}': Not a directory", dest),
        None => return format!("cp: cannot create '{}': No such file or directory", dest),
    };

    // Clone node with new owner
    let new_node = clone_node_with_owner(&source_node, current_user);
    parent.insert(dest_name.clone(), new_node);

    String::new()
}

pub fn mv(args: &[&str]) -> String {
    let options = match options::parse("mv", args, OptionSpec::new(&['f'], &["force", "help"])) {
        Ok(options) => options,
        Err(error) => return error,
    };

    if options.has_help() {
        return "Usage: mv [-f] <source> <destination>\n  -f, --force   do not prompt before overwriting"
            .to_string();
    }

    if options.operands.len() < 2 {
        return "mv: missing destination file operand".into();
    }

    if options.operands.len() > 2 {
        return "mv: too many arguments".into();
    }

    let force = options.has_short('f') || options.has_long("force");

    let mut filesystem = FILESYSTEM.lock().unwrap();
    let current_path = CURRENT_PATH.lock().unwrap();
    let current_user = &*CURRENT_USER;

    let source = options.operands[0].as_str();
    let dest = options.operands[1].as_str();

    let source_path = normalize_path(source, &current_path);
    let dest_path = normalize_path(dest, &current_path);

    if source_path.is_empty() {
        return "mv: cannot move '/': Permission denied".into();
    }

    // First, remove from source
    let source_parent_path = &source_path[..source_path.len() - 1];
    let source_name = &source_path[source_path.len() - 1];

    let source_node = {
        let parent = match get_node_mut(&mut filesystem, source_parent_path) {
            Some(Node::Directory { children, .. }) => children,
            Some(_) => return format!("mv: cannot move '{}': Not a directory", source),
            None => return format!("mv: cannot stat '{}': No such file or directory", source),
        };

        match parent.get(source_name) {
            Some(node) => {
                if node.is_protected() {
                    return format!(
                        "mv: cannot move '{}': Operation not permitted (protected)",
                        source
                    );
                }
                if node.get_owner() != current_user && current_user != "root" {
                    return format!("mv: cannot move '{}': Permission denied", source);
                }
                parent.remove(source_name)
            }
            None => return format!("mv: cannot stat '{}': No such file or directory", source),
        }
    };

    let source_node = match source_node {
        Some(node) => node,
        None => return format!("mv: cannot stat '{}': No such file or directory", source),
    };

    if dest_path.is_empty() {
        return "mv: cannot move to '/': Permission denied".into();
    }

    let dest_parent_path = &dest_path[..dest_path.len() - 1];
    let dest_name = &dest_path[dest_path.len() - 1];

    let parent = match get_node_mut(&mut filesystem, dest_parent_path) {
        Some(Node::Directory { children, .. }) => children,
        Some(_) => return format!("mv: cannot move to '{}': Not a directory", dest),
        None => return format!("mv: cannot move to '{}': No such file or directory", dest),
    };

    if parent.contains_key(dest_name) && !force {
        return format!("mv: cannot move to '{}': File exists", dest);
    }

    parent.insert(dest_name.clone(), source_node);

    String::new()
}

fn clone_node_with_owner(node: &Node, new_owner: &str) -> Node {
    match node {
        Node::File {
            content,
            permissions,
            ..
        } => Node::File {
            content: content.clone(),
            permissions: *permissions,
            owner: new_owner.to_string(),
            protected: false,
        },
        Node::Directory {
            children,
            permissions,
            ..
        } => {
            let new_children: HashMap<String, Node> = children
                .iter()
                .map(|(name, child)| (name.clone(), clone_node_with_owner(child, new_owner)))
                .collect();
            Node::Directory {
                permissions: *permissions,
                owner: new_owner.to_string(),
                protected: false,
                children: new_children,
            }
        }
        Node::Symlink { target, .. } => Node::Symlink {
            target: target.clone(),
            owner: new_owner.to_string(),
        },
    }
}

pub fn nvim(args: &[&str]) -> String {
    let options = match options::parse("nvim", args, OptionSpec::new(&[], &["help"])) {
        Ok(options) => options,
        Err(error) => return error,
    };

    if options.has_help() {
        return "Usage: nvim <file>\nInside editor:\n  i       insert mode\n  Esc     normal mode\n  :w      write\n  :q      quit\n  :wq     write + quit"
            .to_string();
    }

    if options.operands.is_empty() {
        return "nvim: missing file name".to_string();
    }

    if options.operands.len() > 1 {
        return "nvim: too many arguments".to_string();
    }

    format!("__OPEN_EDITOR__:{}", options.operands[0])
}

pub fn uname(args: &[&str]) -> String {
    let options = match options::parse(
        "uname",
        args,
        OptionSpec::new(&['a', 's'], &["all", "kernel-name", "help"]),
    ) {
        Ok(options) => options,
        Err(error) => return error,
    };

    if options.has_help() {
        return "Usage: uname [-a|-s]\n  -a, --all          print all information\n  -s, --kernel-name  print kernel name"
            .to_string();
    }

    if let Err(error) = options::no_args("uname", &options.operands) {
        return error;
    }

    if options.has_short('a') || options.has_long("all") {
        "WASM wasm-host 1.0.0 #1 SMP PREEMPT_DYNAMIC Mon Jan 1 12:00:00 UTC 2025 wasm32 GNU/Linux"
            .to_string()
    } else {
        "WASM".to_string()
    }
}

use crate::commands::filesystem::{
    get_node_at_path, get_node_at_path_mut, normalize_path, Node, CURRENT_PATH, CURRENT_USER,
    FILESYSTEM,
};
use std::collections::HashMap;

pub fn ls(args: &[&str]) -> String {
    let filesystem = FILESYSTEM.lock().unwrap();
    let current_path = CURRENT_PATH.lock().unwrap();

    let mut show_hidden = false;
    let mut long_format = false;
    let mut target_path = None;

    for arg in args {
        if arg.starts_with('-') {
            for c in arg.chars().skip(1) {
                match c {
                    'a' => show_hidden = true,
                    'l' => long_format = true,
                    _ => return format!("ls: invalid option -- '{}'", c),
                }
            }
        } else {
            target_path = Some(*arg);
        }
    }

    let path = if let Some(target) = target_path {
        normalize_path(target, &current_path)
    } else {
        current_path.clone()
    };

    let node = match get_node_at_path(&filesystem, &path) {
        Some(node) => node,
        None => return "ls: cannot access: No such file or directory".into(),
    };

    match node {
        Node::Directory { children, .. } => {
            let mut entries: Vec<_> = children.iter().collect();
            entries.sort_by_key(|(name, _)| name.as_str());

            if long_format {
                let mut output = String::new();
                for (name, node) in &entries {
                    if !show_hidden && name.starts_with('.') {
                        continue;
                    }

                    let (file_type, permissions, size) = match node {
                        Node::Directory { permissions, .. } => ('d', *permissions, 4096),
                        Node::File {
                            permissions,
                            content,
                            ..
                        } => ('-', *permissions, content.len()),
                        Node::Symlink { .. } => ('l', 0o777, 0),
                    };

                    output.push_str(&format!(
                        "{}{:o} 1 objz objz {:>8} {} {}\n",
                        file_type, permissions, size, "Jan  1 12:00", name
                    ));
                }
                output
            } else {
                entries
                    .iter()
                    .filter(|(name, _)| show_hidden || !name.starts_with('.'))
                    .map(|(name, node)| match node {
                        Node::Directory { .. } => format!("{}/", name),
                        Node::File { .. } => name.to_string(),
                        Node::Symlink { .. } => format!("{}@", name),
                    })
                    .collect::<Vec<_>>()
                    .join("  ")
            }
        }
        Node::File { .. } => target_path.unwrap_or(".").to_string(),
        Node::Symlink { target, .. } => format!("-> {}", target),
    }
}

pub fn cd(args: &[&str]) -> String {
    if args.is_empty() {
        {
            let mut path = CURRENT_PATH.lock().unwrap();
            *path = vec!["home".to_string(), "objz".to_string()];
        }
        return String::new();
    }

    let new_path = {
        let filesystem = FILESYSTEM.lock().unwrap();
        let current_path = CURRENT_PATH.lock().unwrap();

        let new_path = normalize_path(args[0], &current_path);

        match get_node_at_path(&filesystem, &new_path) {
            Some(Node::Directory { .. }) => new_path,
            Some(Node::Symlink { target, .. }) => {
                let symlink_path = normalize_path(target, &current_path);
                match get_node_at_path(&filesystem, &symlink_path) {
                    Some(Node::Directory { .. }) => symlink_path,
                    Some(_) => return format!("cd: {}: Not a directory", args[0]),
                    None => return format!("cd: {}: No such file or directory", args[0]),
                }
            }
            Some(_) => return format!("cd: {}: Not a directory", args[0]),
            None => return format!("cd: {}: No such file or directory", args[0]),
        }
    };

    {
        let mut path = CURRENT_PATH.lock().unwrap();
        *path = new_path;
    }
    String::new()
}

pub fn pwd(_: &[&str]) -> String {
    let path = CURRENT_PATH.lock().unwrap();
    if path.is_empty() {
        "/".into()
    } else {
        format!("/{}", path.join("/"))
    }
}

pub fn cat(args: &[&str]) -> String {
    if args.is_empty() {
        return "cat: missing file operand".into();
    }

    let filesystem = FILESYSTEM.lock().unwrap();
    let current_path = CURRENT_PATH.lock().unwrap();

    let mut output = String::new();

    for &filename in args {
        let file_path = normalize_path(filename, &current_path);

        match get_node_at_path(&filesystem, &file_path) {
            Some(Node::File { content, .. }) => {
                output.push_str(content);
                if args.len() > 1 && filename != args[args.len() - 1] {
                    output.push('\n');
                }
            }
            Some(Node::Directory { .. }) => {
                output.push_str(&format!("cat: {}: Is a directory\n", filename));
            }
            Some(Node::Symlink { target, .. }) => {
                output.push_str(&format!(
                    "cat: {}: Symbolic link (points to {})\n",
                    filename, target
                ));
            }
            None => {
                output.push_str(&format!("cat: {}: No such file or directory\n", filename));
            }
        }
    }

    output.trim_end().to_string()
}

pub fn mkdir(args: &[&str]) -> String {
    if args.is_empty() {
        return "mkdir: missing operand".into();
    }

    let mut filesystem = FILESYSTEM.lock().unwrap();
    let current_path = CURRENT_PATH.lock().unwrap();
    let current_user = &*CURRENT_USER;

    for &dirname in args {
        let dir_path = normalize_path(dirname, &current_path);

        if dir_path.is_empty() {
            return "mkdir: cannot create directory '/': File exists".into();
        }

        let parent_path = &dir_path[..dir_path.len() - 1];
        let dir_name = &dir_path[dir_path.len() - 1];

        let parent = match get_node_at_path_mut(&mut filesystem, parent_path) {
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
    if args.is_empty() {
        return "touch: missing file operand".into();
    }

    let mut filesystem = FILESYSTEM.lock().unwrap();
    let current_path = CURRENT_PATH.lock().unwrap();
    let current_user = &*CURRENT_USER;

    for &filename in args {
        let file_path = normalize_path(filename, &current_path);

        if file_path.is_empty() {
            continue;
        }

        let parent_path = &file_path[..file_path.len() - 1];
        let file_name = &file_path[file_path.len() - 1];

        let parent = match get_node_at_path_mut(&mut filesystem, parent_path) {
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

pub fn rm(args: &[&str]) -> String {
    if args.is_empty() {
        return "rm: missing operand".into();
    }

    let mut filesystem = FILESYSTEM.lock().unwrap();
    let current_path = CURRENT_PATH.lock().unwrap();
    let current_user = &*CURRENT_USER;

    let mut recursive = false;
    let mut force = false;
    let mut files = Vec::new();

    for &arg in args {
        if arg.starts_with('-') {
            for c in arg.chars().skip(1) {
                match c {
                    'r' | 'R' => recursive = true,
                    'f' => force = true,
                    _ => return format!("rm: invalid option -- '{}'", c),
                }
            }
        } else {
            files.push(arg);
        }
    }

    for filename in files {
        let file_path = normalize_path(filename, &current_path);

        if file_path.is_empty() {
            if !force {
                return "rm: cannot remove '/': Permission denied".into();
            }
            continue;
        }

        let parent_path = &file_path[..file_path.len() - 1];
        let file_name = &file_path[file_path.len() - 1];

        let parent = match get_node_at_path_mut(&mut filesystem, parent_path) {
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
    let filesystem = FILESYSTEM.lock().unwrap();
    let current_path = CURRENT_PATH.lock().unwrap();

    let start_path = if args.is_empty() {
        current_path.clone()
    } else {
        normalize_path(args[0], &current_path)
    };

    let start_node = match get_node_at_path(&filesystem, &start_path) {
        Some(node) => node,
        None => return "tree: No such file or directory".into(),
    };

    fn build_tree(node: &Node, prefix: &str, _is_last: bool) -> String {
        let mut output = String::new();

        if let Node::Directory { children, .. } = node {
            let mut entries: Vec<_> = children.iter().collect();
            entries.sort_by_key(|(name, _)| name.as_str());

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

                output.push_str(&build_tree(child, &new_prefix, is_last_child));
            }
        }

        output
    }

    let tree_name = if start_path.is_empty() {
        "/".to_string()
    } else {
        start_path.last().unwrap_or(&"/".to_string()).clone()
    };

    format!("{}\n{}", tree_name, build_tree(start_node, "", true))
}

pub fn ln(args: &[&str]) -> String {
    if args.len() < 2 {
        return "ln: missing file operand".into();
    }

    let mut filesystem = FILESYSTEM.lock().unwrap();
    let current_path = CURRENT_PATH.lock().unwrap();
    let current_user = &*CURRENT_USER;

    let (target, link_name) = if args[0] == "-s" {
        if args.len() < 3 {
            return "ln: missing file operand".into();
        }
        (args[1], args[2])
    } else {
        return "ln: hard links not supported in this filesystem".into();
    };

    let link_path = normalize_path(link_name, &current_path);

    if link_path.is_empty() {
        return "ln: cannot create link '/': File exists".into();
    }

    let parent_path = &link_path[..link_path.len() - 1];
    let file_name = &link_path[link_path.len() - 1];

    let parent = match get_node_at_path_mut(&mut filesystem, parent_path) {
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

pub fn uname(args: &[&str]) -> String {
    if args.is_empty() || args[0] == "-s" {
        "WASM".to_string()
    } else if args[0] == "-a" {
        "WASM wasm-host 1.0.0 #1 SMP PREEMPT_DYNAMIC Mon Jan 1 12:00:00 UTC 2025 wasm32 GNU/Linux"
            .to_string()
    } else {
        "uname: invalid option".to_string()
    }
}

use lazy_static::lazy_static;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Mutex;

const PROJECTS_PATH_PARTS: [&str; 3] = ["home", "objz", "projects"];

#[derive(Clone, Debug)]
pub enum Node {
    File {
        content: String,
        permissions: u16,
        owner: String,
        protected: bool,
    },
    Directory {
        children: HashMap<String, Node>,
        permissions: u16,
        owner: String,
        protected: bool,
    },
    Symlink {
        target: String,
        owner: String,
    },
}

#[derive(Clone, Debug)]
pub struct ProjectFileSpec {
    pub file_name: String,
    pub repo_name: String,
    pub repo_url: String,
    pub content: String,
}

#[derive(Clone, Deserialize)]
struct FilesystemContent {
    projects_readme: String,
    about_txt: String,
    contact_txt: String,
    zshrc: String,
    credits_txt: String,
    rust_txt: String,
    boot_log: String,
}

impl Node {
    pub fn is_protected(&self) -> bool {
        match self {
            Node::File { protected, .. } => *protected,
            Node::Directory { protected, .. } => *protected,
            Node::Symlink { .. } => false,
        }
    }

    pub fn get_owner(&self) -> &str {
        match self {
            Node::File { owner, .. } => owner,
            Node::Directory { owner, .. } => owner,
            Node::Symlink { owner, .. } => owner,
        }
    }
}

lazy_static! {
    static ref FILESYSTEM_CONTENT: FilesystemContent =
        serde_json::from_str(include_str!("../../static/content/filesystem.json"))
            .expect("static/content/filesystem.json must be valid");
    pub static ref FILESYSTEM: Mutex<Node> = Mutex::new({
        use Node::*;
        Directory {
            permissions: 0o755,
            owner: "root".to_string(),
            protected: true,
            children: HashMap::from([
                (
                    "home".into(),
                    Directory {
                        permissions: 0o755,
                        owner: "root".to_string(),
                        protected: true,
                        children: HashMap::from([(
                            "objz".into(),
                            Directory {
                                permissions: 0o755,
                                owner: "objz".to_string(),
                                protected: true,
                                children: HashMap::from([
                                    (
                                        "projects".into(),
                                        Directory {
                                            permissions: 0o755,
                                            owner: "objz".to_string(),
                                            protected: true,
                                            children: HashMap::from([
                                                (
                                                    "README.md".into(),
                                                    File {
                                                        content: FILESYSTEM_CONTENT.projects_readme.clone(),
                                                        permissions: 0o644,
                                                        owner: "objz".to_string(),
                                                        protected: true,
                                                    }
                                                ),
                                            ]),
                                        }
                                    ),
                                    (
                                        "about.txt".into(),
                                        File {
                                            content: FILESYSTEM_CONTENT.about_txt.clone(),
                                            permissions: 0o644,
                                            owner: "objz".to_string(),
                                            protected: true,
                                        }
                                    ),
                                    (
                                        "contact.txt".into(),
                                        File {
                                            content: FILESYSTEM_CONTENT.contact_txt.clone(),
                                            permissions: 0o644,
                                            owner: "objz".to_string(),
                                            protected: true,
                                        }
                                    ),
                                    (
                                        ".zshrc".into(),
                                        File {
                                            content: FILESYSTEM_CONTENT.zshrc.clone(),
                                            permissions: 0o644,
                                            owner: "objz".to_string(),
                                            protected: false,
                                        }
                                    ),
                                    (
                                        "credits.txt".into(),
                                        File {
                                            content: FILESYSTEM_CONTENT.credits_txt.clone(),
                                            permissions: 0o644,
                                            owner: "objz".to_string(),
                                            protected: true,
                                        }
                                    ),
                                ]),
                            }
                        )]),
                    }
                ),
                (
                    "etc".into(),
                    Directory {
                        permissions: 0o755,
                        owner: "root".to_string(),
                        protected: true,
                        children: HashMap::from([
                            (
                                "hostname".into(),
                                File {
                                    content: "wasm-host".into(),
                                    permissions: 0o644,
                                    owner: "root".to_string(),
                                    protected: true,
                                }
                            ),
                            (
                                "passwd".into(),
                                File {
                                    content: "root:x:0:0:root:/root:/bin/bash\nobjz:x:1000:1000:objz:/home/objz:/bin/bash\nnobody:x:65534:65534:nobody:/:/usr/bin/nologin".into(),
                                    permissions: 0o644,
                                    owner: "root".to_string(),
                                    protected: true,
                                }
                            ),
                        ]),
                    }
                ),
                (
                    "tmp".into(),
                    Directory {
                        permissions: 0o1777,
                        owner: "root".to_string(),
                        protected: false,
                        children: HashMap::from([
                            (
                                "rust.txt".into(),
                                File {
                                    content: FILESYSTEM_CONTENT.rust_txt.clone(),
                                    permissions: 0o644,
                                    owner: "objz".to_string(),
                                    protected: false,
                                }
                            ),
                        ]),
                    }
                ),
                (
                    "usr".into(),
                    Directory {
                        permissions: 0o755,
                        owner: "root".to_string(),
                        protected: true,
                        children: HashMap::from([
                            (
                                "bin".into(),
                                Directory {
                                    permissions: 0o755,
                                    owner: "root".to_string(),
                                    protected: true,
                                    children: HashMap::new(),
                                }
                            ),
                        ]),
                    }
                ),
                (
                    "var".into(),
                    Directory {
                        permissions: 0o755,
                        owner: "root".to_string(),
                        protected: true,
                        children: HashMap::from([
                            (
                                "log".into(),
                                Directory {
                                    permissions: 0o755,
                                    owner: "root".to_string(),
                                    protected: true,
                                    children: HashMap::from([
                                        (
                                            "boot.log".into(),
                                            File {
                                                content: FILESYSTEM_CONTENT.boot_log.clone(),
                                                permissions: 0o644,
                                                owner: "root".to_string(),
                                                protected: true,
                                            }
                                        ),
                                    ]),
                                }
                            ),
                        ]),
                    }
                ),
            ]),
        }
    });
    pub static ref CURRENT_PATH: Mutex<Vec<String>> =
        Mutex::new(vec!["home".to_string(), "objz".to_string()]);
    pub static ref CURRENT_USER: String = "objz".to_string();
    static ref PROJECT_FILE_ORDER: Mutex<Vec<String>> = Mutex::new(Vec::new());
    static ref PROJECT_FILE_URLS: Mutex<HashMap<String, String>> = Mutex::new(HashMap::new());
}

pub fn is_projects_path(path: &[String]) -> bool {
    path.len() == PROJECTS_PATH_PARTS.len()
        && path
            .iter()
            .zip(PROJECTS_PATH_PARTS)
            .all(|(part, expected)| part == expected)
}

pub fn project_sort_rank(path: &[String], entry_name: &str) -> Option<usize> {
    if !is_projects_path(path) {
        return None;
    }

    PROJECT_FILE_ORDER
        .lock()
        .unwrap()
        .iter()
        .position(|name| name == entry_name)
}

pub fn project_url_for_token(token: &str) -> Option<String> {
    let trimmed = token.trim().trim_matches('/');
    if trimmed.is_empty() {
        return None;
    }

    let key = trimmed.to_lowercase();
    PROJECT_FILE_URLS.lock().unwrap().get(&key).cloned()
}

pub fn project_repo_candidates() -> Vec<String> {
    let order = PROJECT_FILE_ORDER.lock().unwrap();
    let mut candidates = Vec::new();

    for file_name in order.iter() {
        if file_name.eq_ignore_ascii_case("README.md") {
            continue;
        }

        if let Some(repo) = file_name.strip_suffix(".md") {
            candidates.push(repo.to_string());
        }
    }

    candidates
}

pub fn replace_projects_directory(files: &[ProjectFileSpec]) -> Result<(), String> {
    let mut filesystem = FILESYSTEM.lock().unwrap();
    let projects_path: Vec<String> = PROJECTS_PATH_PARTS
        .iter()
        .map(|segment| segment.to_string())
        .collect();

    let projects_node = get_node_mut(&mut filesystem, &projects_path)
        .ok_or_else(|| "projects directory is missing".to_string())?;

    let children = match projects_node {
        Node::Directory { children, .. } => children,
        _ => return Err("projects path is not a directory".to_string()),
    };

    children.clear();

    let mut order = Vec::new();
    let mut url_map = HashMap::new();

    let mut index_lines = vec![
        "Projects folder is now synced from GitHub.".to_string(),
        "Most recently updated repositories are listed first.".to_string(),
        "Use `cat <repo>.md` to read a project document with rendered README.".to_string(),
        String::new(),
    ];

    for file in files {
        order.push(file.file_name.clone());

        let key_file = file.file_name.to_lowercase();
        url_map.insert(key_file, file.repo_url.clone());

        let key_repo = file.repo_name.to_lowercase();
        url_map.insert(key_repo, file.repo_url.clone());

        children.insert(
            file.file_name.clone(),
            Node::File {
                content: file.content.clone(),
                permissions: 0o644,
                owner: "objz".to_string(),
                protected: true,
            },
        );

        index_lines.push(format!("- {} -> {}", file.file_name, file.repo_url));
    }

    children.insert(
        "README.md".to_string(),
        Node::File {
            content: index_lines.join("\n"),
            permissions: 0o644,
            owner: "objz".to_string(),
            protected: true,
        },
    );

    *PROJECT_FILE_ORDER.lock().unwrap() = order;
    *PROJECT_FILE_URLS.lock().unwrap() = url_map;

    Ok(())
}

pub fn normalize_path(path: &str, current: &[String]) -> Vec<String> {
    if path.starts_with('/') {
        let mut result = Vec::new();
        for part in path.split('/').filter(|s| !s.is_empty()) {
            match part {
                "." => continue,
                ".." => {
                    result.pop();
                }
                _ => result.push(part.to_string()),
            }
        }
        result
    } else {
        let mut result = current.to_vec();
        for part in path.split('/').filter(|s| !s.is_empty()) {
            match part {
                "." => continue,
                ".." => {
                    result.pop();
                }
                _ => result.push(part.to_string()),
            }
        }
        result
    }
}

pub fn get_node<'a>(root: &'a Node, path: &[String]) -> Option<&'a Node> {
    let mut current = root;
    for part in path {
        if let Node::Directory { children, .. } = current {
            current = children.get(part)?;
        } else {
            return None;
        }
    }
    Some(current)
}

pub fn get_node_mut<'a>(root: &'a mut Node, path: &[String]) -> Option<&'a mut Node> {
    let mut current = root;
    for part in path {
        if let Node::Directory { children, .. } = current {
            current = children.get_mut(part)?;
        } else {
            return None;
        }
    }
    Some(current)
}

pub fn autocomplete_entries(path: &[String], dirs_only: bool) -> Vec<String> {
    let filesystem = FILESYSTEM.lock().unwrap();

    match get_node(&filesystem, path) {
        Some(Node::Directory { children, .. }) => {
            let mut entries: Vec<String> = children
                .iter()
                .filter_map(|(name, node)| {
                    if dirs_only {
                        match node {
                            Node::Directory { .. } => Some(format!("{}/", name)),
                            _ => None,
                        }
                    } else {
                        match node {
                            Node::Directory { .. } => Some(format!("{}/", name)),
                            Node::File { .. } => Some(name.clone()),
                            Node::Symlink { .. } => Some(name.clone()),
                        }
                    }
                })
                .collect();

            entries.sort();
            entries
        }
        _ => Vec::new(),
    }
}

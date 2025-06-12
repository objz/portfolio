use lazy_static::lazy_static;
use std::collections::HashMap;
use std::sync::Mutex;

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

impl Node {
    pub fn _is_directory(&self) -> bool {
        matches!(self, Node::Directory { .. })
    }

    pub fn _is_file(&self) -> bool {
        matches!(self, Node::File { .. })
    }

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
                                                    "CommandBridge.md".into(),
                                                    File {
                                                        content: "A scripting-based plugin enabling advanced command forwarding between Velocity and Paper Minecraft servers. Supports flexible automation and custom workflows for server administrators.\n\nProject link: [https://github.com/objz/CommandBridge]\n\nStatus: Active development".into(),
                                                        permissions: 0o644,
                                                        owner: "objz".to_string(),
                                                        protected: true,
                                                    }
                                                ),
                                                (
                                                    "mcl.md".into(),
                                                    File {
                                                        content: "A fast, Rust-powered command-line Minecraft launcher focused on performance and simplicity. Designed for direct launching, version management, and mod integration.\n\nProject link: [https://github.com/objz/mcl]\n\nStatus: In development (not finished yet)".into(),
                                                        permissions: 0o644,
                                                        owner: "objz".to_string(),
                                                        protected: true,
                                                    }
                                                ),
                                                (
                                                    "PowerImport.md".into(),
                                                    File {
                                                        content: "An Excel VSTO add-in for importing and synchronizing Power BI queries directly into spreadsheets. Built for seamless integration and efficient data workflows in enterprise environments.\n\nProject link: [https://github.com/objz/PowerImport]\n\nStatus: Completed".into(),
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
                                            content: "Hi, I'm objz and I'm 17 years old.\nMy main skills are:\n\n- Rust (primary)\n- Java (primary)\n- C (occasionally)\n- Web Development (only if absolutely necessary)".into(),
                                            permissions: 0o644,
                                            owner: "objz".to_string(),
                                            protected: true,
                                        }
                                    ),
                                    (
                                        "contact.txt".into(),
                                        File {
                                            content: "GitHub: @objz\nEmail: me@objz.dev\nLocation: Bavaria, Germany\nResponse time: Eventually™".into(),
                                            permissions: 0o644,
                                            owner: "objz".to_string(),
                                            protected: true,
                                        }
                                    ),
                                    (
                                        ".bashrc".into(),
                                        File {
                                            content: "# ~/.bashrc\nexport PS1='\\u@\\h:\\w\\$ '\nalias ll='ls -la'".into(),
                                            permissions: 0o644,
                                            owner: "objz".to_string(),
                                            protected: false,
                                        }
                                    ),
                                    (
                                        "credits.txt".into(),
                                        File {
                                            content: "This site was developed by objz.\n\nBuilt with:\n- Rust and WebAssembly (Wasm)\n- Three.js for 3D rendering\n\n3D model provided by Sketchfab: [https://shorturl.at/OXITb]\nLooping background music from Freesound: [https://shorturl.at/YYufx]\n\nNo warranty, express or implied.".into(),
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
                                    content: "Did you know?\nRust was originally developed by Mozilla.\nThe first stable release was in 2015.".into(),
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
                                                content: "Loading Linux kernel version 6.8.9-wasm-1...\nLoading initial ramdisk (initramfs)...\nStarting systemd-udevd v254.5-1...\nProbing hardware...\nDetected storage device: /dev/nvme0n1\nDetected storage device: /dev/sda\nActivating swap on /dev/sda2...\nMounting root filesystem...\nChecking file system on /dev/sda1...\nMounting /boot...\nMounting /home...\nMounting /var...\nStarting systemd-journald.service...\nStarting systemd-tmpfiles-setup-dev.service...\nStarting systemd-sysctl.service...\nStarting Load Kernel Modules...\nLoading kernel modules: i915 ext4 fuse...\nStarting Network Manager...\nStarting Login Service (systemd-logind)...\nStarting Authorization Manager (polkitd)...\nStarting User Manager for UID 1000...\nStarting Interface...".into(),
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

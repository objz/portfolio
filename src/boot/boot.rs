use crate::terminal::{renderer::LineOptions, Terminal};

pub async fn boot(term: &Terminal) {
    let boot_messages = vec![
        "Loading Linux kernel version 6.8.9-wasm1-1...",
        "Loading initial ramdisk (initramfs)...",
        "Starting systemd-udevd v254.5-1...",
        "Probing hardware...",
        "Detected storage device: /dev/nvme0n1",
        "Detected storage device: /dev/sda",
        "Activating swap on /dev/sda2...",
        "Mounting root filesystem...",
        "Checking file system on /dev/sda1...",
        "Mounting /boot...",
        "Mounting /home...",
        "Mounting /var...",
        "Starting systemd-journald.service...",
        "Starting systemd-tmpfiles-setup-dev.service...",
        "Starting systemd-sysctl.service...",
        "Starting Load Kernel Modules...",
        "Loading kernel modules: i915 ext4 fuse...",
        "Starting Network Manager...",
        "Starting Login Service (systemd-logind)...",
        "Starting Authorization Manager (polkitd)...",
        "Starting User Manager for UID 1000...",
        "Starting Interface...",
    ];

    for msg in boot_messages {
        term.add_line(msg, Some(LineOptions::new().with_boot_animation()))
            .await;
        term.sleep(15).await;
    }
    term.sleep(200).await;
}

pub async fn logo(term: &Terminal) {
    let logo_lines = vec![
        "                                                    ",
        " ░▒▓██████▓▒░░▒▓███████▓▒░       ░▒▓█▓▒░▒▓████████▓▒░ ",
        "░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░      ░▒▓█▓▒░      ░▒▓█▓▒░",
        "░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░      ░▒▓█▓▒░    ░▒▓██▓▒░ ",
        "░▒▓█▓▒░░▒▓█▓▒░▒▓███████▓▒░       ░▒▓█▓▒░  ░▒▓██▓▒░   ",
        "░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░░▒▓██▓▒░     ",
        "░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░       ",
        " ░▒▓██████▓▒░░▒▓███████▓▒░ ░▒▓██████▓▒░░▒▓████████▓▒░",
        "                                                    ",
    ];

    for line in logo_lines {
        term.add_line(
            line,
            Some(LineOptions::new().with_typing(10).with_color("cyan")),
        )
        .await;
        term.sleep(30).await;
    }
}

pub async fn login(term: &Terminal) {
    let login_messages = vec![
        ("WASM Linux 6.8.9 (tty1)", "", "green"),
        ("", "", ""),
        ("login: objz", "", "white"),
        ("password: ", "", "white"),
        ("", "", ""),
        ("Last login: Mon May 27 13:59:36 2025", "", "white"),
        ("Type 'help' for further information", "", "yellow"),
        ("", "", ""),
    ];

    for (msg, _status, color) in login_messages {
        if msg.is_empty() {
            term.add_line("", None).await;
        } else if msg.contains("login:") {
            term.add_line(msg, Some(LineOptions::new().with_typing(50)))
                .await;
        } else if msg.contains("password:") {
            let full = format!("{}••••••••", msg);
            term.add_line(&full, Some(LineOptions::new().with_typing(50)))
                .await;
        } else {
            term.add_line(msg, Some(LineOptions::new().with_color(color)))
                .await;
        }
        term.sleep(60).await;
    }
}

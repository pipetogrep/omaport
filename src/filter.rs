/// Processes that listen on local TCP sockets for desktop/IDE/agent plumbing
/// rather than being user-launched web dev servers.
const EXCLUDED_BINARIES: &[&str] = &[
    "agy",
    "t3code",
    "ollama",
    "code",
    "cursor",
    "electron",
    "chromium",
    "chrome",
    "firefox",
    "brave",
    "spotify",
    "discord",
    "slack",
    "syncthing",
    "tailscaled",
    "sshd",
    "cupsd",
    "pulseaudio",
    "pipewire",
    "quickshell",
    "omarchy-shell",
    "hyprland",
    "dbus-daemon",
    "gpg-agent",
    "ssh-agent",
    "xdg-desktop-portal",
    "tracker",
];

const EXCLUDED_CMDLINE_TOKENS: &[&str] = &[
    "antigravity",
    "language-server",
    "lsp-server",
    "typescript-language-server",
    "eslint-server",
    "tailwindcss-language-server",
    "vscode-server",
    ".vscode",
    "t3code",
    "mcp-server",
];

const EXPLICIT_DEV_FRAMEWORKS: &[(&str, &str)] = &[
    ("vite", "vite"),
    ("next-server", "next"),
    ("next dev", "next"),
    ("next start", "next"),
    ("nuxt", "nuxt"),
    ("astro", "astro"),
    ("svelte-kit", "sveltekit"),
    ("remix", "remix"),
    ("webpack", "webpack"),
    ("esbuild", "esbuild"),
    ("parcel", "parcel"),
    ("storybook", "storybook"),
    ("wrangler", "wrangler"),
    ("uvicorn", "uvicorn"),
    ("gunicorn", "gunicorn"),
    ("flask", "flask"),
    ("manage.py", "django"),
    ("http.server", "python"),
    ("streamlit", "streamlit"),
    ("gradio", "gradio"),
    ("jupyter", "jupyter"),
    ("rails", "rails"),
    ("puma", "puma"),
    ("jekyll", "jekyll"),
    ("artisan", "laravel"),
    ("hugo", "hugo"),
    ("trunk", "trunk"),
    ("live-server", "live-server"),
    ("http-server", "http-server"),
    ("miniserve", "miniserve"),
    ("caddy", "caddy"),
    ("nginx", "nginx"),
    ("browser-sync", "browsersync"),
];

const DEV_RUNTIMES: &[&str] = &[
    "node",
    "node-mainthread",
    "bun",
    "deno",
    "npm",
    "pnpm",
    "yarn",
    "python",
    "python3",
    "ruby",
    "php",
    "cargo",
    "go",
    "air",
    "mix",
    "beam.smp",
    "dotnet",
    "java",
    "gradle",
];

/// Decide whether a listening user process is likely a local dev server.
pub fn classify_dev_server(comm: &str, cmdline: &str, port: u16) -> bool {
    let comm_lc = comm.to_ascii_lowercase();
    let cmd_lc = cmdline.to_ascii_lowercase();

    if EXCLUDED_BINARIES
        .iter()
        .any(|ex| comm_lc == *ex || comm_lc.starts_with(&format!("{ex}-")))
    {
        return false;
    }

    if EXCLUDED_CMDLINE_TOKENS.iter().any(|tok| cmd_lc.contains(tok)) {
        return false;
    }

    // Explicit web/dev frameworks always count, even on unusual ports.
    if EXPLICIT_DEV_FRAMEWORKS
        .iter()
        .any(|(needle, _)| cmd_lc.contains(needle) || comm_lc.contains(needle))
    {
        return true;
    }

    // Ephemeral ports (>= 32768) are usually internal RPC sockets unless explicitly matched above.
    if port >= 32768 {
        return false;
    }

    // Common dev runtimes on non-ephemeral ports.
    if DEV_RUNTIMES
        .iter()
        .any(|rt| comm_lc == *rt || comm_lc.starts_with(&format!("{rt}.")))
    {
        return true;
    }

    // Well-known web / dev port ranges for compiled Go/Rust/Zig/C++ dev servers.
    matches!(
        port,
        80 | 443 | 1234 | 1313 | 3000..=3999 | 4000..=4999 | 5000..=5999 | 6006 | 7000..=7999 | 8000..=8999 | 9000..=9999
    )
}

/// Produce a concise, human-friendly server/framework label.
pub fn detect_label(comm: &str, cmdline: &str) -> String {
    let cmd_lc = cmdline.to_ascii_lowercase();
    let comm_lc = comm.to_ascii_lowercase();

    for (needle, label) in EXPLICIT_DEV_FRAMEWORKS {
        if cmd_lc.contains(needle) || comm_lc.contains(needle) {
            return (*label).to_string();
        }
    }

    if comm_lc.starts_with("node") {
        return "node".to_string();
    }
    if comm_lc.starts_with("python") {
        return "python".to_string();
    }
    if comm_lc == "beam.smp" {
        return "elixir".to_string();
    }

    if !comm.is_empty() {
        comm.to_string()
    } else {
        "server".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_common_dev_servers_and_filters_noise() {
        assert!(classify_dev_server(
            "node-MainThread",
            "node /home/xz/app/node_modules/.bin/vite --port 5173",
            5173
        ));
        assert_eq!(
            detect_label(
                "node-MainThread",
                "node /home/xz/app/node_modules/.bin/vite --port 5173"
            ),
            "vite"
        );

        assert!(classify_dev_server("python3", "python3 -m http.server 8000", 8000));
        assert_eq!(detect_label("python3", "python3 -m http.server 8000"), "python");

        assert!(classify_dev_server("my-axum-api", "./target/debug/my-axum-api", 3000));

        assert!(!classify_dev_server("agy", "/usr/bin/agy --rpc", 35905));
        assert!(!classify_dev_server("t3code", "/usr/bin/t3code", 3773));
        assert!(!classify_dev_server("ollama", "ollama serve", 11434));
        assert!(!classify_dev_server(
            "node",
            "node /home/xz/.vscode/extensions/typescript-language-server",
            41234
        ));
    }
}

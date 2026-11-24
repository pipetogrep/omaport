use std::fs;
use std::path::{Path, PathBuf};

/// Resolve the current Git branch (or short commit SHA on detached HEAD) for `dir`.
/// Walks upward from `dir` to find `.git`, supporting both regular repositories
/// (`.git/` directory) and git worktrees/submodules (`.git` file with `gitdir: ...`).
pub fn resolve_branch(dir: &Path) -> Option<String> {
    let git_dir = find_git_dir(dir)?;
    let head_path = git_dir.join("HEAD");
    let content = fs::read_to_string(head_path).ok()?;
    parse_head(&content)
}

fn find_git_dir(start: &Path) -> Option<PathBuf> {
    let mut current = start;
    // Cap traversal depth so pathological symlinks/mounts cannot loop.
    for _ in 0..64 {
        let candidate = current.join(".git");
        if candidate.is_dir() {
            return Some(candidate);
        }
        if candidate.is_file() {
            let text = fs::read_to_string(&candidate).ok()?;
            let first_line = text.lines().next()?.trim();
            if let Some(raw_path) = first_line.strip_prefix("gitdir:") {
                let raw_path = raw_path.trim();
                let resolved = if Path::new(raw_path).is_absolute() {
                    PathBuf::from(raw_path)
                } else {
                    current.join(raw_path)
                };
                if resolved.is_dir() {
                    return Some(resolved);
                }
            }
            return None;
        }
        current = current.parent()?;
    }
    None
}

pub fn parse_head(content: &str) -> Option<String> {
    let line = content.lines().next()?.trim();
    if line.is_empty() {
        return None;
    }
    if let Some(rest) = line.strip_prefix("ref:") {
        let reference = rest.trim();
        let branch = reference
            .strip_prefix("refs/heads/")
            .or_else(|| reference.strip_prefix("refs/tags/"))
            .unwrap_or(reference);
        if branch.is_empty() {
            None
        } else {
            Some(branch.to_string())
        }
    } else if line.len() >= 7 && line.chars().all(|c| c.is_ascii_hexdigit()) {
        // Detached HEAD: display 7-char short SHA.
        Some(line[..7].to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("omaport_git_{name}_{nanos}"));
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn parses_standard_branch_and_detached_head() {
        assert_eq!(parse_head("ref: refs/heads/main\n"), Some("main".into()));
        assert_eq!(
            parse_head("ref: refs/heads/feat/my-branch\n"),
            Some("feat/my-branch".into())
        );
        assert_eq!(
            parse_head("0123456789abcdef0123456789abcdef01234567\n"),
            Some("0123456".into())
        );
        assert_eq!(parse_head(""), None);
    }

    #[test]
    fn resolves_standard_repo_from_subdirectory() {
        let root = temp_dir("standard");
        fs::create_dir_all(root.join(".git")).unwrap();
        fs::write(root.join(".git/HEAD"), "ref: refs/heads/develop\n").unwrap();
        let nested = root.join("apps/web");
        fs::create_dir_all(&nested).unwrap();

        assert_eq!(resolve_branch(&nested), Some("develop".into()));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn resolves_git_worktree_pointer_file() {
        let root = temp_dir("worktree");
        let real_gitdir = root.join("main-repo/.git/worktrees/feature-wt");
        fs::create_dir_all(&real_gitdir).unwrap();
        fs::write(real_gitdir.join("HEAD"), "ref: refs/heads/feature/wt\n").unwrap();

        let wt = root.join("wt-checkout");
        fs::create_dir_all(&wt).unwrap();
        fs::write(
            wt.join(".git"),
            format!("gitdir: {}\n", real_gitdir.display()),
        )
        .unwrap();

        assert_eq!(resolve_branch(&wt), Some("feature/wt".into()));
        let _ = fs::remove_dir_all(&root);
    }
}

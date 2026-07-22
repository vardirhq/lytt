use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectInfo {
    pub root: PathBuf,
    pub name: String,
    pub kind: String,
}

pub fn detect_project(cwd: Option<&Path>) -> Option<ProjectInfo> {
    let mut current = cwd?;

    loop {
        if current.join("package.json").exists() {
            return Some(project(current, "node"));
        }
        if current.join("Cargo.toml").exists() {
            return Some(project(current, "rust"));
        }
        if current.join("go.mod").exists() {
            return Some(project(current, "go"));
        }
        if current.join("pyproject.toml").exists() {
            return Some(project(current, "python"));
        }
        if current.join(".git").exists() {
            return Some(project(current, "git"));
        }

        current = current.parent()?;
    }
}

fn project(path: &Path, kind: &str) -> ProjectInfo {
    ProjectInfo {
        root: path.to_path_buf(),
        name: path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| path.display().to_string()),
        kind: kind.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_name_uses_directory_name() {
        let info = project(Path::new("/home/chris/code/skald"), "git");

        assert_eq!(info.name, "skald");
        assert_eq!(info.kind, "git");
    }
}

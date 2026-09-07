use std::path::{Path, PathBuf};

use crate::skills::is_valid_name;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Agent {
    pub name: String,
    pub path: PathBuf,
}

pub fn scan_agents(repo_dir: &Path) -> Result<Vec<Agent>, String> {
    let agents_dir = repo_dir.join("agents");
    if !agents_dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut agents = Vec::new();
    for entry in std::fs::read_dir(&agents_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let file_name = entry.file_name().to_string_lossy().into_owned();
        if entry.path().is_dir() || !file_name.ends_with(".md") || file_name == "README.md" {
            continue;
        }
        let name = file_name.trim_end_matches(".md");
        if !is_valid_name(name) {
            return Err(format!("agents/{file_name}: invalid agent name '{name}'"));
        }
        agents.push(Agent {
            name: name.to_string(),
            path: entry.path(),
        });
    }
    agents.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(agents)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil;
    use std::fs;
    use std::path::PathBuf;

    fn temp_repo() -> PathBuf {
        testutil::temp_dir("repo")
    }

    fn write_agent(repo: &Path, file: &str) {
        let agents = repo.join("agents");
        fs::create_dir_all(&agents).unwrap();
        fs::write(agents.join(file), "---\ndescription: d\n---\nBody\n").unwrap();
    }

    #[test]
    fn repo_without_agents_dir_scans_no_agents() {
        let repo = temp_repo();
        let agents = scan_agents(&repo).unwrap();
        assert!(agents.is_empty());
        fs::remove_dir_all(&repo).unwrap();
    }

    #[test]
    fn top_level_md_files_are_scanned_as_agents() {
        let repo = temp_repo();
        write_agent(&repo, "reviewer.md");
        write_agent(&repo, "developer.md");
        let agents = scan_agents(&repo).unwrap();
        let names: Vec<_> = agents.iter().map(|a| a.name.clone()).collect();
        assert_eq!(names, vec!["developer".to_string(), "reviewer".to_string()]);
        assert_eq!(agents[0].path, repo.join("agents").join("developer.md"));
        fs::remove_dir_all(&repo).unwrap();
    }

    #[test]
    fn subdirectories_and_non_md_files_are_ignored() {
        let repo = temp_repo();
        let agents_dir = repo.join("agents");
        fs::create_dir_all(agents_dir.join("nested")).unwrap();
        fs::write(agents_dir.join("notes.txt"), "x").unwrap();
        fs::write(agents_dir.join(".DS_Store"), "x").unwrap();
        write_agent(&repo, "developer.md");
        let agents = scan_agents(&repo).unwrap();
        assert_eq!(agents.len(), 1);
        fs::remove_dir_all(&repo).unwrap();
    }

    #[test]
    fn readme_md_is_ignored() {
        let repo = temp_repo();
        write_agent(&repo, "README.md");
        write_agent(&repo, "developer.md");
        let agents = scan_agents(&repo).unwrap();
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0].name, "developer");
        fs::remove_dir_all(&repo).unwrap();
    }

    #[test]
    fn uppercase_md_extension_is_ignored() {
        let repo = temp_repo();
        write_agent(&repo, "developer.MD");
        let agents = scan_agents(&repo).unwrap();
        assert!(agents.is_empty());
        fs::remove_dir_all(&repo).unwrap();
    }

    #[test]
    fn invalid_agent_name_is_an_error() {
        let repo = temp_repo();
        write_agent(&repo, "My Agent.md");
        let err = scan_agents(&repo).unwrap_err();
        assert!(
            err.contains("My Agent") && err.contains("invalid agent name"),
            "error should name the offender: {err}"
        );
        fs::remove_dir_all(&repo).unwrap();
    }

    #[test]
    fn hidden_md_file_is_an_error() {
        let repo = temp_repo();
        write_agent(&repo, ".hidden.md");
        let err = scan_agents(&repo).unwrap_err();
        assert!(err.contains(".hidden"), "error should name the file: {err}");
        fs::remove_dir_all(&repo).unwrap();
    }
}

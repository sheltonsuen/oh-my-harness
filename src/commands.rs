use std::path::{Path, PathBuf};

use crate::skills::is_valid_name;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command {
    pub name: String,
    pub path: PathBuf,
}

pub fn scan_commands(repo_dir: &Path) -> Result<Vec<Command>, String> {
    let commands_dir = repo_dir.join("commands");
    if !commands_dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut commands = Vec::new();
    for entry in std::fs::read_dir(&commands_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let file_name = entry.file_name().to_string_lossy().into_owned();
        if entry.path().is_dir() || !file_name.ends_with(".md") || file_name == "README.md" {
            continue;
        }
        let name = file_name.trim_end_matches(".md");
        if !is_valid_name(name) {
            return Err(format!(
                "commands/{file_name}: invalid command name '{name}'"
            ));
        }
        commands.push(Command {
            name: name.to_string(),
            path: entry.path(),
        });
    }
    commands.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(commands)
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

    fn write_command(repo: &Path, file: &str) {
        testutil::write_command(repo, file, "---\ndescription: d\n---\nBody\n");
    }

    #[test]
    fn repo_without_commands_dir_scans_no_commands() {
        let repo = temp_repo();
        let commands = scan_commands(&repo).unwrap();
        assert!(commands.is_empty());
        fs::remove_dir_all(&repo).unwrap();
    }

    #[test]
    fn top_level_md_files_are_scanned_as_commands() {
        let repo = temp_repo();
        write_command(&repo, "wrap-up.md");
        write_command(&repo, "kick-off.md");
        let commands = scan_commands(&repo).unwrap();
        let names: Vec<_> = commands.iter().map(|c| c.name.clone()).collect();
        assert_eq!(names, vec!["kick-off".to_string(), "wrap-up".to_string()]);
        assert_eq!(commands[0].path, repo.join("commands").join("kick-off.md"));
        fs::remove_dir_all(&repo).unwrap();
    }

    #[test]
    fn subdirectories_and_non_md_files_are_ignored() {
        let repo = temp_repo();
        let commands_dir = repo.join("commands");
        fs::create_dir_all(commands_dir.join("nested")).unwrap();
        fs::write(commands_dir.join("notes.txt"), "x").unwrap();
        fs::write(commands_dir.join(".DS_Store"), "x").unwrap();
        write_command(&repo, "kick-off.md");
        let commands = scan_commands(&repo).unwrap();
        assert_eq!(commands.len(), 1);
        fs::remove_dir_all(&repo).unwrap();
    }

    #[test]
    fn readme_md_is_ignored() {
        let repo = temp_repo();
        write_command(&repo, "README.md");
        write_command(&repo, "kick-off.md");
        let commands = scan_commands(&repo).unwrap();
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].name, "kick-off");
        fs::remove_dir_all(&repo).unwrap();
    }

    #[test]
    fn uppercase_md_extension_is_ignored() {
        let repo = temp_repo();
        write_command(&repo, "kick-off.MD");
        let commands = scan_commands(&repo).unwrap();
        assert!(commands.is_empty());
        fs::remove_dir_all(&repo).unwrap();
    }

    #[test]
    fn invalid_command_name_is_an_error() {
        let repo = temp_repo();
        write_command(&repo, "My Command.md");
        let err = scan_commands(&repo).unwrap_err();
        assert!(
            err.contains("My Command") && err.contains("invalid command name"),
            "error should name the offender: {err}"
        );
        fs::remove_dir_all(&repo).unwrap();
    }

    #[test]
    fn hidden_md_file_is_an_error() {
        let repo = temp_repo();
        write_command(&repo, ".hidden.md");
        let err = scan_commands(&repo).unwrap_err();
        assert!(err.contains(".hidden"), "error should name the file: {err}");
        fs::remove_dir_all(&repo).unwrap();
    }
}

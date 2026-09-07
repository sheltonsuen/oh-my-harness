use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Skill {
    pub name: String,
    pub path: PathBuf,
}

pub(crate) fn is_valid_name(name: &str) -> bool {
    let mut prev_was_dash = true; // reject a leading '-'
    if name.is_empty() {
        return false;
    }
    for c in name.chars() {
        if c == '-' {
            if prev_was_dash {
                return false;
            }
            prev_was_dash = true;
        } else if c.is_ascii_lowercase() || c.is_ascii_digit() {
            prev_was_dash = false;
        } else {
            return false;
        }
    }
    !prev_was_dash // reject a trailing '-'
}

fn parse_frontmatter(content: &str) -> Option<(String, String)> {
    let mut lines = content.lines();
    if lines.next()?.trim() != "---" {
        return None;
    }
    let mut name = None;
    let mut description = None;
    let mut closed = false;
    for line in lines {
        if line.trim() == "---" {
            closed = true;
            break;
        }
        if let Some((key, value)) = line.split_once(':') {
            let value = value.trim();
            if value.is_empty() {
                continue;
            }
            match key.trim() {
                "name" => name = Some(value.to_string()),
                "description" => description = Some(value.to_string()),
                _ => {}
            }
        }
    }
    if !closed {
        return None;
    }
    Some((name?, description?))
}

pub fn scan_skills(repo_dir: &std::path::Path) -> Result<Vec<Skill>, String> {
    let skills_dir = repo_dir.join("skills");
    if !skills_dir.is_dir() {
        return Err(format!(
            "no skills/ directory found in {}",
            repo_dir.display()
        ));
    }
    let mut skills = Vec::new();
    for entry in std::fs::read_dir(&skills_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        if entry.path().is_dir() {
            if !entry.path().join("SKILL.md").is_file() {
                return Err(format!(
                    "skills/{} is missing SKILL.md",
                    entry.file_name().to_string_lossy()
                ));
            }
            let folder = entry.file_name().to_string_lossy().into_owned();
            let content = std::fs::read_to_string(entry.path().join("SKILL.md"))
                .map_err(|e| format!("skills/{folder}: {e}"))?;
            let (name, _description) = parse_frontmatter(&content).ok_or_else(|| {
                format!("skills/{folder}: invalid or missing frontmatter (name, description)")
            })?;
            if !is_valid_name(&name) {
                return Err(format!("skills/{folder}: invalid skill name '{name}'"));
            }
            if name != folder {
                return Err(format!(
                    "skills/{folder}: frontmatter name '{name}' does not match folder name"
                ));
            }
            skills.push(Skill {
                name,
                path: entry.path(),
            });
        }
    }
    Ok(skills)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil;
    use std::fs;
    use std::path::{Path, PathBuf};

    fn temp_repo() -> PathBuf {
        testutil::temp_dir("repo")
    }

    fn write_skill(repo: &Path, folder: &str, content: &str) {
        testutil::write_skill(repo, folder, content);
    }

    #[test]
    fn missing_skills_dir_is_an_error() {
        let repo = temp_repo();
        let err = scan_skills(&repo).unwrap_err();
        assert!(
            err.contains("skills"),
            "error should mention skills/: {err}"
        );
        fs::remove_dir_all(&repo).unwrap();
    }

    #[test]
    fn valid_skill_folder_is_returned_with_name_and_path() {
        let repo = temp_repo();
        write_skill(
            &repo,
            "define",
            "---\nname: define\ndescription: Grill the user.\n---\nBody\n",
        );
        let skills = scan_skills(&repo).unwrap();
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "define");
        assert_eq!(skills[0].path, repo.join("skills").join("define"));
        fs::remove_dir_all(&repo).unwrap();
    }

    #[test]
    fn folder_without_skill_md_is_an_error() {
        let repo = temp_repo();
        fs::create_dir_all(repo.join("skills").join("broken")).unwrap();
        let err = scan_skills(&repo).unwrap_err();
        assert!(
            err.contains("broken"),
            "error should name the offending folder: {err}"
        );
        fs::remove_dir_all(&repo).unwrap();
    }

    #[test]
    fn loose_files_in_skills_dir_are_ignored() {
        let repo = temp_repo();
        write_skill(&repo, "define", "---\nname: define\ndescription: d\n---\n");
        fs::write(repo.join("skills").join("README.md"), "# nope").unwrap();
        let skills = scan_skills(&repo).unwrap();
        assert_eq!(skills.len(), 1);
        fs::remove_dir_all(&repo).unwrap();
    }

    #[test]
    fn unterminated_frontmatter_is_an_error() {
        let repo = temp_repo();
        write_skill(
            &repo,
            "define",
            "---\nname: define\ndescription: d\nBody never closes",
        );
        let err = scan_skills(&repo).unwrap_err();
        assert!(err.contains("define"), "error should name the skill: {err}");
        fs::remove_dir_all(&repo).unwrap();
    }

    #[test]
    fn empty_required_frontmatter_field_is_an_error() {
        let repo = temp_repo();
        write_skill(&repo, "define", "---\nname: define\ndescription:\n---\n");
        let err = scan_skills(&repo).unwrap_err();
        assert!(err.contains("define"), "error should name the skill: {err}");
        fs::remove_dir_all(&repo).unwrap();
    }

    #[test]
    fn frontmatter_name_mismatching_folder_is_an_error() {
        let repo = temp_repo();
        write_skill(&repo, "define", "---\nname: grill\ndescription: d\n---\n");
        let err = scan_skills(&repo).unwrap_err();
        assert!(
            err.contains("define"),
            "error should name the folder: {err}"
        );
        assert!(
            err.contains("grill"),
            "error should name the frontmatter: {err}"
        );
        fs::remove_dir_all(&repo).unwrap();
    }

    #[test]
    fn skill_name_with_invalid_characters_is_an_error() {
        for bad in ["Bad", "has space", "-lead", "1-2-"] {
            let repo = temp_repo();
            write_skill(
                &repo,
                bad,
                &format!("---\nname: {bad}\ndescription: d\n---\n"),
            );
            let err = scan_skills(&repo).unwrap_err();
            assert!(err.contains(bad), "error should name '{bad}': {err}");
            fs::remove_dir_all(&repo).unwrap();
        }
    }

    #[test]
    fn missing_required_frontmatter_field_is_an_error() {
        let repo = temp_repo();
        write_skill(&repo, "define", "---\nname: define\n---\n");
        assert!(scan_skills(&repo).unwrap_err().contains("define"));
        fs::remove_dir_all(&repo).unwrap();

        let repo = temp_repo();
        write_skill(&repo, "define", "no frontmatter here");
        assert!(scan_skills(&repo).unwrap_err().contains("define"));
        fs::remove_dir_all(&repo).unwrap();
    }
}

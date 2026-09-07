use std::fs;
use std::path::{Path, PathBuf};

use crate::skills::scan_skills;

pub trait Git {
    fn clone_shallow(&self, url: &str, dest: &Path) -> Result<(), String>;
}

pub struct SystemGit;

impl Git for SystemGit {
    fn clone_shallow(&self, url: &str, dest: &Path) -> Result<(), String> {
        let output = std::process::Command::new("git")
            .args(["clone", "--depth", "1", url])
            .arg(dest)
            .output()
            .map_err(|e| format!("failed to run git: {e}"))?;
        if output.status.success() {
            Ok(())
        } else {
            Err(format!(
                "git clone failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ))
        }
    }
}

#[derive(Debug, Default)]
pub struct Report {
    pub installed: Vec<String>,
    pub skipped: Vec<String>,
}

pub fn install(
    repo: &str,
    skills_dest: &Path,
    force: bool,
    git: &dyn Git,
) -> Result<Report, String> {
    let repo_path = Path::new(repo);
    let (source, tmp) = if repo_path.is_dir() {
        (repo_path.to_path_buf(), Option::<PathBuf>::None)
    } else {
        let tmp = std::env::temp_dir().join(format!(
            "omh-clone-{}-{}",
            std::process::id(),
            std::time::UNIX_EPOCH.elapsed().unwrap().as_nanos()
        ));
        fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
        if let Err(e) = git.clone_shallow(repo, &tmp) {
            let _ = fs::remove_dir_all(&tmp);
            return Err(e);
        }
        (tmp.clone(), Some(tmp))
    };

    let result = install_skills(&source, repo, skills_dest, force);
    if let Some(tmp) = &tmp {
        let _ = fs::remove_dir_all(tmp);
    }
    result
}

fn install_skills(
    source: &Path,
    label: &str,
    skills_dest: &Path,
    force: bool,
) -> Result<Report, String> {
    let skills =
        scan_skills(source).map_err(|e| e.replace(&source.display().to_string(), label))?;
    let mut report = Report::default();
    fs::create_dir_all(skills_dest).map_err(|e| e.to_string())?;
    for skill in &skills {
        let target = skills_dest.join(&skill.name);
        if target.exists() && !force {
            report.skipped.push(skill.name.clone());
            continue;
        }
        // Copy into a staging dir first so a failed copy never destroys an existing skill.
        let staging = skills_dest.join(format!(".{}.staging-{}", skill.name, std::process::id()));
        let _ = fs::remove_dir_all(&staging);
        if let Err(e) = copy_dir_all(&skill.path, &staging) {
            let _ = fs::remove_dir_all(&staging);
            return Err(e);
        }
        if target.exists()
            && let Err(e) = fs::remove_dir_all(&target)
        {
            let _ = fs::remove_dir_all(&staging);
            return Err(e.to_string());
        }
        match fs::rename(&staging, &target) {
            Ok(()) => report.installed.push(skill.name.clone()),
            Err(e) => {
                let _ = fs::remove_dir_all(&staging);
                return Err(e.to_string());
            }
        }
    }
    Ok(report)
}

fn copy_dir_all(from: &Path, to: &Path) -> Result<(), String> {
    fs::create_dir_all(to).map_err(|e| e.to_string())?;
    for entry in fs::read_dir(from).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let dest = to.join(entry.file_name());
        if entry.file_type().map_err(|e| e.to_string())?.is_dir() {
            copy_dir_all(&entry.path(), &dest)?;
        } else {
            fs::copy(entry.path(), dest).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    struct NoGit;
    impl Git for NoGit {
        fn clone_shallow(&self, _url: &str, _dest: &Path) -> Result<(), String> {
            panic!("git must not be used for a local-dir repo")
        }
    }

    fn temp_dir(tag: &str) -> PathBuf {
        crate::testutil::temp_dir(tag)
    }

    fn write_skill(repo: &Path, folder: &str) {
        crate::testutil::write_skill(
            repo,
            folder,
            &format!("---\nname: {folder}\ndescription: d\n---\nBody\n"),
        );
    }

    struct FakeGit {
        called_with: std::cell::RefCell<Option<(String, PathBuf)>>,
    }
    impl FakeGit {
        fn new() -> Self {
            Self {
                called_with: std::cell::RefCell::new(None),
            }
        }
    }
    impl Git for FakeGit {
        fn clone_shallow(&self, url: &str, dest: &Path) -> Result<(), String> {
            *self.called_with.borrow_mut() = Some((url.to_string(), dest.to_path_buf()));
            write_skill(dest, "demo");
            Ok(())
        }
    }

    #[test]
    fn url_repo_is_shallow_cloned_installed_and_tmp_cleaned() {
        let git = FakeGit::new();
        let dest = temp_dir("url-dest");
        let report = install("https://example.com/repo.git", &dest, false, &git).unwrap();
        let (url, clone_dir) = {
            let guard = git.called_with.borrow();
            guard.clone().expect("clone called")
        };
        assert_eq!(url, "https://example.com/repo.git");
        assert_eq!(report.installed, vec!["demo".to_string()]);
        assert!(dest.join("demo").join("SKILL.md").is_file());
        assert!(!clone_dir.exists(), "tmp clone dir must be cleaned up");
        fs::remove_dir_all(&dest).unwrap();
    }

    #[test]
    fn existing_skill_is_skipped_and_left_untouched() {
        let repo = temp_dir("repo2");
        write_skill(&repo, "demo");
        let dest = temp_dir("dest2");
        fs::create_dir_all(dest.join("demo")).unwrap();
        fs::write(dest.join("demo").join("SKILL.md"), "my local edits").unwrap();
        let report = install(repo.to_str().unwrap(), &dest, false, &NoGit).unwrap();
        assert_eq!(report.skipped, vec!["demo".to_string()]);
        assert!(report.installed.is_empty());
        assert_eq!(
            fs::read_to_string(dest.join("demo").join("SKILL.md")).unwrap(),
            "my local edits"
        );
        fs::remove_dir_all(&repo).unwrap();
        fs::remove_dir_all(&dest).unwrap();
    }

    #[test]
    fn force_overwrites_existing_skill() {
        let repo = temp_dir("repo3");
        write_skill(&repo, "demo");
        let dest = temp_dir("dest3");
        fs::create_dir_all(dest.join("demo")).unwrap();
        fs::write(dest.join("demo").join("stale.md"), "old").unwrap();
        let report = install(repo.to_str().unwrap(), &dest, true, &NoGit).unwrap();
        assert_eq!(report.installed, vec!["demo".to_string()]);
        assert!(
            !dest.join("demo").join("stale.md").exists(),
            "old folder must be replaced"
        );
        assert!(dest.join("demo").join("SKILL.md").is_file());
        fs::remove_dir_all(&repo).unwrap();
        fs::remove_dir_all(&dest).unwrap();
    }

    #[test]
    fn one_invalid_skill_aborts_whole_install() {
        let repo = temp_dir("repo4");
        write_skill(&repo, "good");
        fs::create_dir_all(repo.join("skills").join("bad")).unwrap();
        let dest = temp_dir("dest4");
        let err = install(repo.to_str().unwrap(), &dest, false, &NoGit).unwrap_err();
        assert!(err.contains("bad"), "error should name offender: {err}");
        assert!(
            !dest.join("good").exists(),
            "nothing may be installed when validation fails"
        );
        fs::remove_dir_all(&repo).unwrap();
        fs::remove_dir_all(&dest).unwrap();
    }

    #[test]
    fn tmp_clone_is_cleaned_when_install_fails_after_clone() {
        struct BrokenGit {
            clone_dir: std::cell::RefCell<Option<PathBuf>>,
        }
        impl Git for BrokenGit {
            fn clone_shallow(&self, _url: &str, dest: &Path) -> Result<(), String> {
                *self.clone_dir.borrow_mut() = Some(dest.to_path_buf());
                fs::create_dir_all(dest.join("skills").join("broken")).unwrap();
                Ok(())
            }
        }
        let git = BrokenGit {
            clone_dir: std::cell::RefCell::new(None),
        };
        let dest = temp_dir("fail-dest");
        let err = install("https://example.com/x.git", &dest, false, &git).unwrap_err();
        assert!(err.contains("broken"), "error should name offender: {err}");
        let clone_dir = git.clone_dir.borrow().clone().unwrap();
        assert!(!clone_dir.exists(), "tmp clone must be cleaned on failure");
        fs::remove_dir_all(&dest).unwrap();
    }

    #[test]
    fn missing_skills_error_names_the_repo_not_the_tmp_dir() {
        struct EmptyGit {
            clone_dir: std::cell::RefCell<Option<PathBuf>>,
        }
        impl Git for EmptyGit {
            fn clone_shallow(&self, _url: &str, dest: &Path) -> Result<(), String> {
                *self.clone_dir.borrow_mut() = Some(dest.to_path_buf());
                Ok(())
            }
        }
        let git = EmptyGit {
            clone_dir: std::cell::RefCell::new(None),
        };
        let dest = temp_dir("label-dest");
        let err = install("https://example.com/y.git", &dest, false, &git).unwrap_err();
        assert!(
            err.contains("https://example.com/y.git"),
            "error should name the repo the user passed: {err}"
        );
        assert!(
            !err.contains("omh-clone"),
            "error must not leak the tmp clone dir: {err}"
        );
        let clone_dir = git.clone_dir.borrow().clone().unwrap();
        assert!(!clone_dir.exists(), "tmp clone must be cleaned on failure");
        fs::remove_dir_all(&dest).unwrap();
    }

    #[test]
    fn failed_force_copy_leaves_existing_skill_untouched() {
        let repo = temp_dir("repo5");
        write_skill(&repo, "demo");
        fs::set_permissions(
            repo.join("skills/demo/references"),
            fs::Permissions::from_mode(0o000),
        )
        .unwrap();
        let dest = temp_dir("dest5");
        fs::create_dir_all(dest.join("demo")).unwrap();
        fs::write(dest.join("demo").join("SKILL.md"), "precious").unwrap();
        let result = install(repo.to_str().unwrap(), &dest, true, &NoGit);
        fs::set_permissions(
            repo.join("skills/demo/references"),
            fs::Permissions::from_mode(0o755),
        )
        .unwrap();
        assert!(result.is_err(), "unreadable source must fail the install");
        assert_eq!(
            fs::read_to_string(dest.join("demo").join("SKILL.md")).unwrap(),
            "precious",
            "a failed force copy must not destroy the existing skill"
        );
        let leftovers: Vec<_> = fs::read_dir(&dest)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|n| n.starts_with('.'))
            .collect();
        assert!(
            leftovers.is_empty(),
            "no staging dirs left behind: {leftovers:?}"
        );
        fs::remove_dir_all(&repo).unwrap();
        fs::remove_dir_all(&dest).unwrap();
    }

    #[test]
    fn local_dir_repo_copies_whole_skill_folders() {
        let repo = temp_dir("repo");
        write_skill(&repo, "define");
        let dest = temp_dir("dest");
        let report = install(repo.to_str().unwrap(), &dest, false, &NoGit).unwrap();
        assert_eq!(report.installed, vec!["define".to_string()]);
        assert!(dest.join("define").join("SKILL.md").is_file());
        assert!(
            dest.join("define")
                .join("references")
                .join("x.md")
                .is_file(),
            "supporting files must be copied"
        );
        fs::remove_dir_all(&repo).unwrap();
        fs::remove_dir_all(&dest).unwrap();
    }
}

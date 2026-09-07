use std::fs;
use std::path::{Path, PathBuf};

use crate::agents::scan_agents;
use crate::commands::scan_commands;
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
    pub agents_installed: Vec<String>,
    pub agents_skipped: Vec<String>,
    pub commands_installed: Vec<String>,
    pub commands_skipped: Vec<String>,
}

/// Where a repo's payload gets installed: opencode's skills, agents and
/// commands roots for either the global or a project-local configuration.
#[derive(Debug, Clone)]
pub struct Destinations {
    pub skills: PathBuf,
    pub agents: PathBuf,
    pub commands: PathBuf,
}

pub fn install(
    repo: &str,
    dests: &Destinations,
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

    let result = install_all(&source, repo, dests, force);
    if let Some(tmp) = &tmp {
        let _ = fs::remove_dir_all(tmp);
    }
    result
}

fn install_all(
    source: &Path,
    label: &str,
    dests: &Destinations,
    force: bool,
) -> Result<Report, String> {
    let strip = |e: String| e.replace(&source.display().to_string(), label);
    if !["skills", "agents", "commands"]
        .iter()
        .any(|dir| source.join(dir).is_dir())
    {
        return Err(format!(
            "no skills/, agents/, or commands/ directory found in {label}"
        ));
    }
    let skills = scan_skills(source).map_err(strip)?;
    let agents = scan_agents(source).map_err(strip)?;
    let commands = scan_commands(source).map_err(strip)?;
    let mut report = Report::default();
    fs::create_dir_all(&dests.skills).map_err(|e| e.to_string())?;
    for skill in &skills {
        let target = dests.skills.join(&skill.name);
        if target.exists() && !force {
            report.skipped.push(skill.name.clone());
            continue;
        }
        // Copy into a staging dir first so a failed copy never destroys an existing skill.
        let staging = dests
            .skills
            .join(format!(".{}.staging-{}", skill.name, std::process::id()));
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
    if !agents.is_empty() {
        fs::create_dir_all(&dests.agents).map_err(|e| e.to_string())?;
    }
    for agent in &agents {
        let target = dests.agents.join(format!("{}.md", agent.name));
        if target.exists() && !force {
            report.agents_skipped.push(agent.name.clone());
            continue;
        }
        // Stage beside the target and rename over it so a failed copy never destroys an existing agent.
        let staging =
            dests
                .agents
                .join(format!(".{}.staging-{}.md", agent.name, std::process::id()));
        let _ = fs::remove_file(&staging);
        if let Err(e) = fs::copy(&agent.path, &staging) {
            let _ = fs::remove_file(&staging);
            return Err(e.to_string());
        }
        match fs::rename(&staging, &target) {
            Ok(()) => report.agents_installed.push(agent.name.clone()),
            Err(e) => {
                let _ = fs::remove_file(&staging);
                return Err(e.to_string());
            }
        }
    }
    if !commands.is_empty() {
        fs::create_dir_all(&dests.commands).map_err(|e| e.to_string())?;
    }
    for command in &commands {
        let target = dests.commands.join(format!("{}.md", command.name));
        if target.exists() && !force {
            report.commands_skipped.push(command.name.clone());
            continue;
        }
        // Stage beside the target and rename over it so a failed copy never destroys an existing command.
        let staging = dests.commands.join(format!(
            ".{}.staging-{}.md",
            command.name,
            std::process::id()
        ));
        let _ = fs::remove_file(&staging);
        if let Err(e) = fs::copy(&command.path, &staging) {
            let _ = fs::remove_file(&staging);
            return Err(e.to_string());
        }
        match fs::rename(&staging, &target) {
            Ok(()) => report.commands_installed.push(command.name.clone()),
            Err(e) => {
                let _ = fs::remove_file(&staging);
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
        let report = install(
            "https://example.com/repo.git",
            &dests(&dest, &temp_dir("adest"), &commands_dest("adest-cmds")),
            false,
            &git,
        )
        .unwrap();
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
        let report = install(
            repo.to_str().unwrap(),
            &dests(&dest, &temp_dir("adest"), &commands_dest("adest-cmds")),
            false,
            &NoGit,
        )
        .unwrap();
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
        let report = install(
            repo.to_str().unwrap(),
            &dests(&dest, &temp_dir("adest"), &commands_dest("adest-cmds")),
            true,
            &NoGit,
        )
        .unwrap();
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
        let err = install(
            repo.to_str().unwrap(),
            &dests(&dest, &temp_dir("adest"), &commands_dest("adest-cmds")),
            false,
            &NoGit,
        )
        .unwrap_err();
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
        let err = install(
            "https://example.com/x.git",
            &dests(&dest, &temp_dir("adest"), &commands_dest("adest-cmds")),
            false,
            &git,
        )
        .unwrap_err();
        assert!(err.contains("broken"), "error should name offender: {err}");
        let clone_dir = git.clone_dir.borrow().clone().unwrap();
        assert!(!clone_dir.exists(), "tmp clone must be cleaned on failure");
        fs::remove_dir_all(&dest).unwrap();
    }

    #[test]
    fn missing_payload_dirs_error_names_the_repo_not_the_tmp_dir() {
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
        let err = install(
            "https://example.com/y.git",
            &dests(&dest, &temp_dir("adest"), &commands_dest("adest-cmds")),
            false,
            &git,
        )
        .unwrap_err();
        assert!(
            err.contains("https://example.com/y.git"),
            "error should name the repo the user passed: {err}"
        );
        assert!(
            err.contains("skills/") && err.contains("agents/") && err.contains("commands/"),
            "error should name all three payload dirs: {err}"
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
        let result = install(
            repo.to_str().unwrap(),
            &dests(&dest, &temp_dir("adest"), &commands_dest("adest-cmds")),
            true,
            &NoGit,
        );
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
        let report = install(
            repo.to_str().unwrap(),
            &dests(&dest, &temp_dir("agents-dest"), &commands_dest("cmds")),
            false,
            &NoGit,
        )
        .unwrap();
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

    fn agents_dest(tag: &str) -> PathBuf {
        temp_dir(tag).join("agents")
    }

    fn commands_dest(tag: &str) -> PathBuf {
        temp_dir(tag).join("commands")
    }

    fn dests(skills: &Path, agents: &Path, commands: &Path) -> Destinations {
        Destinations {
            skills: skills.to_path_buf(),
            agents: agents.to_path_buf(),
            commands: commands.to_path_buf(),
        }
    }

    #[test]
    fn agent_files_are_installed_and_reported() {
        let repo = temp_dir("repo-agents");
        write_skill(&repo, "define");
        crate::testutil::write_agent(&repo, "developer.md", "---\ndescription: d\n---\nBody\n");
        let dest = temp_dir("dest-agents");
        let adest = agents_dest("agents-dest-agents");
        let report = install(
            repo.to_str().unwrap(),
            &dests(&dest, &adest, &commands_dest("cdest")),
            false,
            &NoGit,
        )
        .unwrap();
        assert_eq!(report.installed, vec!["define".to_string()]);
        assert_eq!(report.agents_installed, vec!["developer".to_string()]);
        assert!(report.agents_skipped.is_empty());
        assert!(adest.join("developer.md").is_file());
        fs::remove_dir_all(&repo).unwrap();
        fs::remove_dir_all(&dest).unwrap();
        fs::remove_dir_all(&adest).unwrap();
    }

    #[test]
    fn missing_agents_dir_installs_no_agents_and_creates_nothing() {
        let repo = temp_dir("repo-noagents");
        write_skill(&repo, "define");
        let dest = temp_dir("dest-noagents");
        let adest = agents_dest("agents-dest-noagents");
        let report = install(
            repo.to_str().unwrap(),
            &dests(&dest, &adest, &commands_dest("cdest")),
            false,
            &NoGit,
        )
        .unwrap();
        assert!(report.agents_installed.is_empty());
        assert!(!adest.exists(), "agents dest must not be created");
        fs::remove_dir_all(&repo).unwrap();
        fs::remove_dir_all(&dest).unwrap();
    }

    #[test]
    fn existing_agent_is_skipped_and_left_untouched() {
        let repo = temp_dir("repo-agskip");
        write_skill(&repo, "define");
        crate::testutil::write_agent(&repo, "developer.md", "new content");
        let dest = temp_dir("dest-agskip");
        let adest = agents_dest("agents-dest-agskip");
        fs::create_dir_all(&adest).unwrap();
        fs::write(adest.join("developer.md"), "my local edits").unwrap();
        let report = install(
            repo.to_str().unwrap(),
            &dests(&dest, &adest, &commands_dest("cdest")),
            false,
            &NoGit,
        )
        .unwrap();
        assert_eq!(report.agents_skipped, vec!["developer".to_string()]);
        assert!(report.agents_installed.is_empty());
        assert_eq!(
            fs::read_to_string(adest.join("developer.md")).unwrap(),
            "my local edits"
        );
        fs::remove_dir_all(&repo).unwrap();
        fs::remove_dir_all(&dest).unwrap();
        fs::remove_dir_all(&adest).unwrap();
    }

    #[test]
    fn force_overwrites_existing_agent() {
        let repo = temp_dir("repo-agforce");
        write_skill(&repo, "define");
        crate::testutil::write_agent(&repo, "developer.md", "new content");
        let dest = temp_dir("dest-agforce");
        let adest = agents_dest("agents-dest-agforce");
        fs::create_dir_all(&adest).unwrap();
        fs::write(adest.join("developer.md"), "old").unwrap();
        let report = install(
            repo.to_str().unwrap(),
            &dests(&dest, &adest, &commands_dest("cdest")),
            true,
            &NoGit,
        )
        .unwrap();
        assert_eq!(report.agents_installed, vec!["developer".to_string()]);
        assert_eq!(
            fs::read_to_string(adest.join("developer.md")).unwrap(),
            "new content"
        );
        let leftovers: Vec<_> = fs::read_dir(&adest)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|n| n.starts_with('.'))
            .collect();
        assert!(leftovers.is_empty(), "no staging files left: {leftovers:?}");
        fs::remove_dir_all(&repo).unwrap();
        fs::remove_dir_all(&dest).unwrap();
        fs::remove_dir_all(&adest).unwrap();
    }

    #[test]
    fn invalid_agent_aborts_before_anything_is_installed() {
        let repo = temp_dir("repo-agbad");
        write_skill(&repo, "define");
        crate::testutil::write_agent(&repo, "Bad Name.md", "x");
        let dest = temp_dir("dest-agbad");
        let adest = agents_dest("agents-dest-agbad");
        let err = install(
            repo.to_str().unwrap(),
            &dests(&dest, &adest, &commands_dest("cdest")),
            false,
            &NoGit,
        )
        .unwrap_err();
        assert!(
            err.contains("Bad Name"),
            "error should name offender: {err}"
        );
        assert!(
            !dest.join("define").exists(),
            "no skills may be installed when an agent is invalid"
        );
        assert!(!adest.exists());
        fs::remove_dir_all(&repo).unwrap();
        fs::remove_dir_all(&dest).unwrap();
    }

    #[test]
    fn failed_force_agent_copy_leaves_existing_agent_untouched() {
        let repo = temp_dir("repo-agfail");
        write_skill(&repo, "define");
        crate::testutil::write_agent(&repo, "developer.md", "new content");
        fs::set_permissions(
            repo.join("agents").join("developer.md"),
            fs::Permissions::from_mode(0o000),
        )
        .unwrap();
        let dest = temp_dir("dest-agfail");
        let adest = agents_dest("agents-dest-agfail");
        fs::create_dir_all(&adest).unwrap();
        fs::write(adest.join("developer.md"), "precious").unwrap();
        let result = install(
            repo.to_str().unwrap(),
            &dests(&dest, &adest, &commands_dest("cdest")),
            true,
            &NoGit,
        );
        fs::set_permissions(
            repo.join("agents").join("developer.md"),
            fs::Permissions::from_mode(0o644),
        )
        .unwrap();
        assert!(result.is_err(), "unreadable source must fail the install");
        assert_eq!(
            fs::read_to_string(adest.join("developer.md")).unwrap(),
            "precious",
            "a failed force copy must not destroy the existing agent"
        );
        let leftovers: Vec<_> = fs::read_dir(&adest)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|n| n.starts_with('.'))
            .collect();
        assert!(leftovers.is_empty(), "no staging files left: {leftovers:?}");
        fs::remove_dir_all(&repo).unwrap();
        fs::remove_dir_all(&dest).unwrap();
        fs::remove_dir_all(&adest).unwrap();
    }

    #[test]
    fn command_files_are_installed_and_reported() {
        let repo = temp_dir("repo-cmds");
        write_skill(&repo, "define");
        crate::testutil::write_command(&repo, "kick-off.md", "---\ndescription: d\n---\nBody\n");
        let dest = temp_dir("dest-cmds");
        let cdest = commands_dest("cmds-dest-cmds");
        let report = install(
            repo.to_str().unwrap(),
            &dests(&dest, &agents_dest("agents-dest-cmds"), &cdest),
            false,
            &NoGit,
        )
        .unwrap();
        assert_eq!(report.installed, vec!["define".to_string()]);
        assert_eq!(report.commands_installed, vec!["kick-off".to_string()]);
        assert!(report.commands_skipped.is_empty());
        assert!(cdest.join("kick-off.md").is_file());
        fs::remove_dir_all(&repo).unwrap();
        fs::remove_dir_all(&dest).unwrap();
        fs::remove_dir_all(&cdest).unwrap();
    }

    #[test]
    fn missing_commands_dir_installs_no_commands_and_creates_nothing() {
        let repo = temp_dir("repo-nocmds");
        write_skill(&repo, "define");
        let dest = temp_dir("dest-nocmds");
        let cdest = commands_dest("cmds-dest-nocmds");
        let report = install(
            repo.to_str().unwrap(),
            &dests(&dest, &agents_dest("agents-dest-nocmds"), &cdest),
            false,
            &NoGit,
        )
        .unwrap();
        assert!(report.commands_installed.is_empty());
        assert!(!cdest.exists(), "commands dest must not be created");
        fs::remove_dir_all(&repo).unwrap();
        fs::remove_dir_all(&dest).unwrap();
    }

    #[test]
    fn existing_command_is_skipped_and_left_untouched() {
        let repo = temp_dir("repo-cmdskip");
        write_skill(&repo, "define");
        crate::testutil::write_command(&repo, "kick-off.md", "new content");
        let dest = temp_dir("dest-cmdskip");
        let cdest = commands_dest("cmds-dest-cmdskip");
        fs::create_dir_all(&cdest).unwrap();
        fs::write(cdest.join("kick-off.md"), "my local edits").unwrap();
        let report = install(
            repo.to_str().unwrap(),
            &dests(&dest, &agents_dest("agents-dest-cmdskip"), &cdest),
            false,
            &NoGit,
        )
        .unwrap();
        assert_eq!(report.commands_skipped, vec!["kick-off".to_string()]);
        assert!(report.commands_installed.is_empty());
        assert_eq!(
            fs::read_to_string(cdest.join("kick-off.md")).unwrap(),
            "my local edits"
        );
        fs::remove_dir_all(&repo).unwrap();
        fs::remove_dir_all(&dest).unwrap();
        fs::remove_dir_all(&cdest).unwrap();
    }

    #[test]
    fn force_overwrites_existing_command() {
        let repo = temp_dir("repo-cmdforce");
        write_skill(&repo, "define");
        crate::testutil::write_command(&repo, "kick-off.md", "new content");
        let dest = temp_dir("dest-cmdforce");
        let cdest = commands_dest("cmds-dest-cmdforce");
        fs::create_dir_all(&cdest).unwrap();
        fs::write(cdest.join("kick-off.md"), "old").unwrap();
        let report = install(
            repo.to_str().unwrap(),
            &dests(&dest, &agents_dest("agents-dest-cmdforce"), &cdest),
            true,
            &NoGit,
        )
        .unwrap();
        assert_eq!(report.commands_installed, vec!["kick-off".to_string()]);
        assert_eq!(
            fs::read_to_string(cdest.join("kick-off.md")).unwrap(),
            "new content"
        );
        let leftovers: Vec<_> = fs::read_dir(&cdest)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|n| n.starts_with('.'))
            .collect();
        assert!(leftovers.is_empty(), "no staging files left: {leftovers:?}");
        fs::remove_dir_all(&repo).unwrap();
        fs::remove_dir_all(&dest).unwrap();
        fs::remove_dir_all(&cdest).unwrap();
    }

    #[test]
    fn invalid_command_aborts_before_anything_is_installed() {
        let repo = temp_dir("repo-cmdbad");
        write_skill(&repo, "define");
        crate::testutil::write_command(&repo, "Bad Name.md", "x");
        let dest = temp_dir("dest-cmdbad");
        let cdest = commands_dest("cmds-dest-cmdbad");
        let err = install(
            repo.to_str().unwrap(),
            &dests(&dest, &agents_dest("agents-dest-cmdbad"), &cdest),
            false,
            &NoGit,
        )
        .unwrap_err();
        assert!(
            err.contains("Bad Name"),
            "error should name offender: {err}"
        );
        assert!(
            !dest.join("define").exists(),
            "no skills may be installed when a command is invalid"
        );
        assert!(!cdest.exists());
        fs::remove_dir_all(&repo).unwrap();
        fs::remove_dir_all(&dest).unwrap();
    }

    #[test]
    fn repo_with_only_commands_dir_installs_commands() {
        let repo = temp_dir("repo-cmdsonly");
        crate::testutil::write_command(&repo, "kick-off.md", "---\ndescription: d\n---\nBody\n");
        let dest = temp_dir("dest-cmdsonly");
        let cdest = commands_dest("cmds-dest-cmdsonly");
        let report = install(
            repo.to_str().unwrap(),
            &dests(&dest, &agents_dest("agents-dest-cmdsonly"), &cdest),
            false,
            &NoGit,
        )
        .unwrap();
        assert_eq!(report.commands_installed, vec!["kick-off".to_string()]);
        assert!(report.installed.is_empty());
        assert!(report.agents_installed.is_empty());
        fs::remove_dir_all(&repo).unwrap();
        fs::remove_dir_all(&dest).unwrap();
        fs::remove_dir_all(&cdest).unwrap();
    }

    #[test]
    fn empty_but_present_payload_dirs_pass_the_contract() {
        let repo = temp_dir("repo-emptydirs");
        fs::create_dir_all(repo.join("skills")).unwrap();
        let dest = temp_dir("dest-emptydirs");
        let cdest = commands_dest("cmds-dest-emptydirs");
        let report = install(
            repo.to_str().unwrap(),
            &dests(&dest, &agents_dest("agents-dest-emptydirs"), &cdest),
            false,
            &NoGit,
        )
        .unwrap();
        assert!(report.installed.is_empty());
        assert!(report.agents_installed.is_empty());
        assert!(report.commands_installed.is_empty());
        assert!(!cdest.exists());
        fs::remove_dir_all(&repo).unwrap();
        fs::remove_dir_all(&dest).unwrap();
    }

    #[test]
    fn failed_force_command_copy_leaves_existing_command_untouched() {
        let repo = temp_dir("repo-cmddfail");
        write_skill(&repo, "define");
        crate::testutil::write_command(&repo, "kick-off.md", "new content");
        fs::set_permissions(
            repo.join("commands").join("kick-off.md"),
            fs::Permissions::from_mode(0o000),
        )
        .unwrap();
        let dest = temp_dir("dest-cmddfail");
        let cdest = commands_dest("cmds-dest-cmddfail");
        fs::create_dir_all(&cdest).unwrap();
        fs::write(cdest.join("kick-off.md"), "precious").unwrap();
        let result = install(
            repo.to_str().unwrap(),
            &dests(&dest, &agents_dest("agents-dest-cmddfail"), &cdest),
            true,
            &NoGit,
        );
        fs::set_permissions(
            repo.join("commands").join("kick-off.md"),
            fs::Permissions::from_mode(0o644),
        )
        .unwrap();
        assert!(result.is_err(), "unreadable source must fail the install");
        assert_eq!(
            fs::read_to_string(cdest.join("kick-off.md")).unwrap(),
            "precious",
            "a failed force copy must not destroy the existing command"
        );
        let leftovers: Vec<_> = fs::read_dir(&cdest)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|n| n.starts_with('.'))
            .collect();
        assert!(leftovers.is_empty(), "no staging files left: {leftovers:?}");
        fs::remove_dir_all(&repo).unwrap();
        fs::remove_dir_all(&dest).unwrap();
        fs::remove_dir_all(&cdest).unwrap();
    }
}

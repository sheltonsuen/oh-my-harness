/// opencode's global skills root is ~/.config/opencode/skills while its
/// project-local root is .opencode (a deliberate asymmetry).
pub fn resolve_skills_dir(
    project_dir: Option<&std::path::Path>,
    home: &std::path::Path,
) -> std::path::PathBuf {
    match project_dir {
        Some(dir) => dir.join(".opencode").join("skills"),
        None => home.join(".config").join("opencode").join("skills"),
    }
}

/// opencode's global agents root is ~/.config/opencode/agents while its
/// project-local root is .opencode/agents.
pub fn resolve_agents_dir(
    project_dir: Option<&std::path::Path>,
    home: &std::path::Path,
) -> std::path::PathBuf {
    match project_dir {
        Some(dir) => dir.join(".opencode").join("agents"),
        None => home.join(".config").join("opencode").join("agents"),
    }
}

/// opencode's global commands root is ~/.config/opencode/commands while its
/// project-local root is .opencode/commands.
pub fn resolve_commands_dir(
    project_dir: Option<&std::path::Path>,
    home: &std::path::Path,
) -> std::path::PathBuf {
    match project_dir {
        Some(dir) => dir.join(".opencode").join("commands"),
        None => home.join(".config").join("opencode").join("commands"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn commands_global_installs_to_config_opencode_commands() {
        let dir = resolve_commands_dir(None, Path::new("/home/u"));
        assert_eq!(dir, Path::new("/home/u/.config/opencode/commands"));
    }

    #[test]
    fn commands_local_installs_to_dot_opencode_commands() {
        let dir = resolve_commands_dir(Some(Path::new("/proj")), Path::new("/home/u"));
        assert_eq!(dir, Path::new("/proj/.opencode/commands"));
    }

    #[test]
    fn global_installs_to_config_opencode_skills() {
        let dir = resolve_skills_dir(None, Path::new("/home/u"));
        assert_eq!(dir, Path::new("/home/u/.config/opencode/skills"));
    }

    #[test]
    fn local_installs_to_dot_opencode_skills() {
        let dir = resolve_skills_dir(Some(Path::new("/proj")), Path::new("/home/u"));
        assert_eq!(dir, Path::new("/proj/.opencode/skills"));
    }

    #[test]
    fn agents_global_installs_to_config_opencode_agents() {
        let dir = resolve_agents_dir(None, Path::new("/home/u"));
        assert_eq!(dir, Path::new("/home/u/.config/opencode/agents"));
    }

    #[test]
    fn agents_local_installs_to_dot_opencode_agents() {
        let dir = resolve_agents_dir(Some(Path::new("/proj")), Path::new("/home/u"));
        assert_eq!(dir, Path::new("/proj/.opencode/agents"));
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

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
}

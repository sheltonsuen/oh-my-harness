#[derive(Clone, Copy, PartialEq, Eq, Debug, clap::ValueEnum)]
pub enum Agent {
    Opencode,
    Codex,
}

/// Two hard-coded maps (global vs local):
///
/// - global: no agent -> ~/.agents/skills, opencode -> ~/.config/opencode/skills,
///   codex -> ~/.codex/skills
/// - local: <dir>/.agents/skills, <dir>/.opencode/skills, <dir>/.codex/skills
///
/// Note the deliberate asymmetry: opencode's native global root is
/// ~/.config/opencode while its project-local root is .opencode.
pub fn resolve_skills_dir(
    agent: Option<Agent>,
    project_dir: Option<&std::path::Path>,
    home: &std::path::Path,
) -> std::path::PathBuf {
    match project_dir {
        Some(dir) => {
            let dot_dir = match agent {
                None => ".agents",
                Some(Agent::Opencode) => ".opencode",
                Some(Agent::Codex) => ".codex",
            };
            dir.join(dot_dir).join("skills")
        }
        None => match agent {
            None => home.join(".agents").join("skills"),
            Some(Agent::Opencode) => home.join(".config").join("opencode").join("skills"),
            Some(Agent::Codex) => home.join(".codex").join("skills"),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn no_agent_global_installs_to_dot_agents_skills() {
        let dir = resolve_skills_dir(None, None, Path::new("/home/u"));
        assert_eq!(dir, Path::new("/home/u/.agents/skills"));
    }

    #[test]
    fn opencode_agent_global_installs_to_config_opencode_skills() {
        let dir = resolve_skills_dir(Some(Agent::Opencode), None, Path::new("/home/u"));
        assert_eq!(dir, Path::new("/home/u/.config/opencode/skills"));
    }

    #[test]
    fn codex_agent_global_installs_to_dot_codex_skills() {
        let dir = resolve_skills_dir(Some(Agent::Codex), None, Path::new("/home/u"));
        assert_eq!(dir, Path::new("/home/u/.codex/skills"));
    }

    #[test]
    fn project_dir_local_rules() {
        let p = Path::new("/proj");
        assert_eq!(
            resolve_skills_dir(None, Some(p), Path::new("/home/u")),
            Path::new("/proj/.agents/skills")
        );
        assert_eq!(
            resolve_skills_dir(Some(Agent::Opencode), Some(p), Path::new("/home/u")),
            Path::new("/proj/.opencode/skills")
        );
        assert_eq!(
            resolve_skills_dir(Some(Agent::Codex), Some(p), Path::new("/home/u")),
            Path::new("/proj/.codex/skills")
        );
    }
}

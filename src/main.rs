mod agents;
mod install;
mod skills;
mod target;
#[cfg(test)]
mod testutil;

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

const DEFAULT_REPO: &str = "https://github.com/sheltonsuen/oh-my-harness";

#[derive(Parser)]
#[command(name = "omh", about = "Manage agent skills across your harness")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Install skills and agents from a repo into opencode's roots
    Install(InstallArgs),
}

#[derive(Args)]
struct InstallArgs {
    /// Repo to install from: git URL or local directory (default: oh-my-harness)
    repo: Option<String>,

    /// Install project-local into this project root instead of globally
    #[arg(short = 'd', long = "dir")]
    dir: Option<PathBuf>,

    /// Overwrite skills and agents that already exist
    #[arg(long)]
    force: bool,
}

fn home_dir() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

fn run() -> Result<(), String> {
    let cli = Cli::parse();
    match cli.command {
        Command::Install(args) => {
            let repo = args.repo.as_deref().unwrap_or(DEFAULT_REPO);
            let home = home_dir()?;
            let dest = target::resolve_skills_dir(args.dir.as_deref(), &home);
            let agents_dest = target::resolve_agents_dir(args.dir.as_deref(), &home);
            let report =
                install::install(repo, &dest, &agents_dest, args.force, &install::SystemGit)?;
            for name in &report.installed {
                println!("installed skill {name} -> {}", dest.join(name).display());
            }
            for name in &report.agents_installed {
                println!(
                    "installed agent {name} -> {}",
                    agents_dest.join(format!("{name}.md")).display()
                );
            }
            for name in &report.skipped {
                eprintln!("skipped skill {name} (already exists, use --force)");
            }
            for name in &report.agents_skipped {
                eprintln!("skipped agent {name} (already exists, use --force)");
            }
            if report.installed.is_empty()
                && report.skipped.is_empty()
                && report.agents_installed.is_empty()
                && report.agents_skipped.is_empty()
            {
                println!("no skills or agents found in {repo}");
            }
            Ok(())
        }
    }
}

fn main() {
    match run() {
        Ok(()) => {}
        Err(err) => {
            eprintln!("error: {err}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn bare_install_parses_with_everything_default() {
        let cli = Cli::try_parse_from(["omh", "install"]).unwrap();
        let Command::Install(args) = &cli.command;
        assert!(args.repo.is_none());
        assert!(args.dir.is_none());
        assert!(!args.force);
    }

    #[test]
    fn install_flags_and_repo_parse() {
        let cli =
            Cli::try_parse_from(["omh", "install", "-d", "proj", "--force", "https://x/y.git"])
                .unwrap();
        let Command::Install(args) = &cli.command;
        assert_eq!(args.dir.as_deref(), Some(std::path::Path::new("proj")));
        assert!(args.force);
        assert_eq!(args.repo.as_deref(), Some("https://x/y.git"));
    }

    #[test]
    fn agent_flag_is_no_longer_accepted() {
        let err = Cli::try_parse_from(["omh", "install", "-a", "opencode"])
            .err()
            .unwrap();
        assert!(err.to_string().contains("unexpected argument"));
        let err = Cli::try_parse_from(["omh", "install", "--agent", "codex"])
            .err()
            .unwrap();
        assert!(err.to_string().contains("unexpected argument"));
    }

    #[test]
    fn default_repo_is_the_harness_url() {
        assert_eq!(DEFAULT_REPO, "https://github.com/sheltonsuen/oh-my-harness");
    }
}

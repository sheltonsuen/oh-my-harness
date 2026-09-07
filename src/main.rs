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
    /// Install skills from a repo into your agent skill roots
    Install(InstallArgs),
}

#[derive(Args)]
struct InstallArgs {
    /// Repo to install from: git URL or local directory (default: oh-my-harness)
    repo: Option<String>,

    /// Target agent; without it skills go to the shared .agents/skills
    #[arg(short = 'a', long = "agent")]
    agent: Option<target::Agent>,

    /// Install project-local into this project root instead of globally
    #[arg(short = 'd', long = "dir")]
    dir: Option<PathBuf>,

    /// Overwrite skills that already exist
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
            let dest = target::resolve_skills_dir(args.agent, args.dir.as_deref(), &home);
            let report = install::install(repo, &dest, args.force, &install::SystemGit)?;
            for name in &report.installed {
                println!("installed {name} -> {}", dest.join(name).display());
            }
            for name in &report.skipped {
                eprintln!("skipped {name} (already exists, use --force)");
            }
            if report.installed.is_empty() && report.skipped.is_empty() {
                println!("no skills found in {repo}");
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
        assert!(args.agent.is_none());
        assert!(args.dir.is_none());
        assert!(!args.force);
    }

    #[test]
    fn install_flags_and_repo_parse() {
        let cli = Cli::try_parse_from([
            "omh",
            "install",
            "-a",
            "opencode",
            "-d",
            "proj",
            "--force",
            "https://x/y.git",
        ])
        .unwrap();
        let Command::Install(args) = &cli.command;
        assert_eq!(args.agent, Some(target::Agent::Opencode));
        assert_eq!(args.dir.as_deref(), Some(std::path::Path::new("proj")));
        assert!(args.force);
        assert_eq!(args.repo.as_deref(), Some("https://x/y.git"));
    }

    #[test]
    fn default_repo_is_the_harness_url() {
        assert_eq!(DEFAULT_REPO, "https://github.com/sheltonsuen/oh-my-harness");
    }
}

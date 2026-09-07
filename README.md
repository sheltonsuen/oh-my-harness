[![CI](https://github.com/sheltonsuen/oh-my-harness/actions/workflows/ci.yml/badge.svg)](https://github.com/sheltonsuen/oh-my-harness/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/oh-my-harness.svg)](https://crates.io/crates/oh-my-harness)
[![Crates.io Downloads](https://img.shields.io/crates/d/oh-my-harness.svg)](https://crates.io/crates/oh-my-harness)
[![Homebrew](https://img.shields.io/github/v/release/sheltonsuen/oh-my-harness?label=homebrew&logo=homebrew)](https://github.com/sheltonsuen/homebrew-oh-my-harness)

# oh-my-harness

## Installation

### Homebrew (macOS)

```sh
brew install sheltonsuen/homebrew-oh-my-harness/omh
```

### Cargo (all platforms)

```sh
cargo install oh-my-harness
```

## Usage

```sh
omh install [repo] [-d <project-dir>] [--force]
```

`omh install` installs everything a repo ships into [opencode](https://opencode.ai):

- `skills/<name>/` folders → `~/.config/opencode/skills/` (or `<project>/.opencode/skills/` with `-d`)
- top-level `agents/*.md` files → `~/.config/opencode/agents/` (or `<project>/.opencode/agents/` with `-d`)

`repo` is a git URL or local directory, and defaults to the oh-my-harness repo. Existing skills and agents are skipped unless `--force` overwrites them. A repo without an `agents/` directory just installs its skills.

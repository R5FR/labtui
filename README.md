# labtui

A fast terminal UI for Git and GitLab, forked from [gitui](https://github.com/gitui-org/gitui).

labtui adds a native **Merge Requests** tab that talks to the GitLab API directly from your terminal — no browser needed.

## Features

- All standard gitui features:
  - Keyboard-only control with context-sensitive help
  - Stage, unstage, revert and reset files, hunks and lines
  - Commit, amend (with hook support: `pre-commit`, `commit-msg`, `post-commit`, `prepare-commit-msg`)
  - Stash (save, pop, apply, drop, inspect)
  - Push / Fetch to / from remote
  - Branch management (create, rename, delete, checkout, remotes)
  - Browse and search commit log, diff committed changes
  - Submodule support
  - GPG commit signing
  - Async git API for fluid, non-blocking control

- **GitLab integration:**
  - Merge Request list tab (opened, draft, merged, closed)
  - Pipeline status badge per MR
  - Token stored securely in the system keyring
  - Auto-detected from the git remote URL — no config needed if the remote is GitLab

## Build

**Requirements:**

- Rust / Cargo ≥ 1.88 — [Install Rust](https://www.rust-lang.org/tools/install)
- To build the OpenSSL dependency: a C compiler and Perl ≥ 5.12
- Python (invocable as `python`) to run the full test suite

```sh
cargo build --release
```

The binary is at `target/release/labtui`.

## GitLab setup

labtui auto-detects GitLab remotes from your repo's remote URL. On first launch in a GitLab repo, it will prompt you to enter a **personal access token** with `read_api` scope. The token is then stored in the system keyring.

You can also set the token via environment variable:

```sh
export GITLAB_TOKEN=your_token
```

## Usage

```sh
labtui
```

Use the `Tab` / `Shift+Tab` keys to navigate between tabs. The **Merge Requests** tab appears automatically when a GitLab remote is detected.

## Key Bindings

Key bindings can be customized. See [KEY_CONFIG.md](KEY_CONFIG.md) for vim-style bindings and other options.

## Color Theme

labtui works on both light and dark terminals. See [THEMES.md](THEMES.md) to customize colors.

## License

MIT — see [LICENSE.md](LICENSE.md).

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
  - Issues tab — list open issues, board view by label (`b`), create (`n`) and close (`c`) them
  - Pipeline status badge per MR
  - Token stored securely in the system keyring
  - Auto-detected from the git remote URL — no config needed if the remote is GitLab

  The underlying `asyncgitlab` crate also covers MR actions (merge / approve /
  rebase / comment), issue & MR notes, and pipelines/jobs (list / trace /
  retry / cancel); see [`asyncgitlab/ROADMAP.md`](asyncgitlab/ROADMAP.md) for
  what is exposed in the UI today.

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

labtui auto-detects GitLab remotes from your repo's remote URL. On first launch in a GitLab repo, it will prompt you to enter a **personal access token**. Use the `api` scope if you want the write actions (creating/closing issues, MR actions); `read_api` is enough for read-only browsing. The token is then stored in the system keyring.

You can also set the token via environment variable:

```sh
export GITLAB_TOKEN=your_token
```

## Usage

```sh
labtui
```

Use the `Tab` / `Shift+Tab` keys (or the number keys) to navigate between tabs. The **Merge Requests** (`6`) and **Issues** (`7`) tabs appear automatically when a GitLab remote is detected. In the Issues tab, press `Enter` to open an issue's detail and comment thread (`n` to add a comment, `c` to close it, `Esc` to go back), `n` to create an issue, `c` to close the selected one, `r` to refresh, and `b` to toggle between the list and the column board view (use `←`/`→` to move between board columns).

## Key Bindings

Key bindings can be customized. See [KEY_CONFIG.md](KEY_CONFIG.md) for vim-style bindings and other options.

## Color Theme

labtui works on both light and dark terminals. See [THEMES.md](THEMES.md) to customize colors.

## License

MIT — see [LICENSE.md](LICENSE.md).

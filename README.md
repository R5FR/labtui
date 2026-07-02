<div align="center">

<img src="assets/labtui-dark.svg#gh-dark-mode-only" width="280" alt="labtui" />
<img src="assets/labtui-light.svg#gh-light-mode-only" width="280" alt="labtui" />

### The terminal cockpit for Git and GitLab

Stage hunks, ship merge requests, and watch CI go green — without ever leaving the keyboard.

[![rust](https://img.shields.io/badge/rust-1.88%2B-ff4c00?logo=rust&logoColor=white&labelColor=0d0d0d)](https://www.rust-lang.org)
[![license](https://img.shields.io/badge/license-MIT-ff4c00?labelColor=0d0d0d)](LICENSE.md)
[![status](https://img.shields.io/badge/status-active-ff4c00?labelColor=0d0d0d)](https://github.com/R5FR/labtui)
[![gitlab](https://img.shields.io/badge/GitLab-native-ff4c00?logo=gitlab&logoColor=white&labelColor=0d0d0d)](#gitlab-setup)

</div>

<br/>

<div align="center">

<video src="https://github.com/user-attachments/assets/3109c1ab-306f-4b1a-a791-aba1e14a6d40" controls muted width="820">
  Your browser can't play this video — grab it directly: <a href="assets/labtui-promo.mp4">assets/labtui-promo.mp4</a>
</video>

<sub>▶ full walkthrough — Git, Merge Requests, Issues &amp; boards, CI/CD</sub>

</div>

<br/>

> [!NOTE]
> **labtui is a fork of [gitui](https://github.com/gitui-org/gitui)**, rebuilt around a single idea: your GitLab workflow shouldn't require a browser tab. Everything below is native, keyboard-driven, and async — the UI never blocks on the network.

---

## Contents

- [Why labtui](#why-labtui)
- [Features](#features)
- [Install](#install)
- [GitLab setup](#gitlab-setup)
- [Usage](#usage)
- [Key bindings](#key-bindings)
- [Color themes](#color-themes)
- [GitLab coverage](#gitlab-coverage)
- [License](#license)

---

## Why labtui

Most Git TUIs stop at `git status`. labtui keeps going: merge requests, issue boards, and pipeline logs render right next to your diff, in the same keystroke-driven interface, with the same async engine so nothing ever freezes waiting on a network call.

| | |
|---|---|
| **Keyboard-only** | Every action — stage, merge, approve, retry a pipeline — is one keypress away |
| **Async core** | Git and GitLab calls run off the main thread; the UI never blocks |
| **Zero config** | GitLab tabs appear automatically the moment a GitLab remote is detected |
| **No plaintext secrets** | Tokens live in the OS keyring, never on disk |

---

## Features

### Git

The complete workflow, keyboard-first:

- Stage, unstage, revert and reset — files, hunks, or individual lines
- Commit and amend with full hook support (`pre-commit`, `commit-msg`, `post-commit`, `prepare-commit-msg`)
- Stash — save, pop, apply, drop, inspect
- Push / fetch to and from remote
- Branch management — create, rename, delete, checkout, remote tracking
- Browse and search the commit log, diff committed changes
- Submodule support
- GPG commit signing
- Async engine — the UI never freezes

### GitLab

<div align="center">

| Tab | Key | What you can do |
|:---:|:---:|---|
| **Merge Requests** | `6` | List MRs with live CI badge · open detail + discussion thread · view diff · merge · approve/unapprove · rebase · close/reopen · comment · edit labels · open in browser |
| **Issues** | `7` | List issues · board view by label (`[` / `]` to switch boards) · open detail + comment thread · create · close/reopen · comment · edit labels · filter |
| **CI/CD** | `8` | Browse pipelines → jobs → job trace · retry/cancel pipeline or job · trigger a new pipeline · per-commit CI status · open in browser |

</div>

GitLab tabs appear **automatically** the moment a GitLab remote is detected — gitlab.com or self-hosted, no config file needed.

---

## Install

**Requirements**

- Rust / Cargo ≥ 1.88 — [install Rust](https://www.rust-lang.org/tools/install)
- A C compiler and Perl ≥ 5.12 *(only for the vendored OpenSSL fallback)*
- Python (invocable as `python`) — to run the full test suite

**Install from crates.io** (recommended)

```sh
cargo install labtui --locked
```

**Build from source**

```sh
cargo build --release
```

The binary is written to `target/release/labtui`.

> [!TIP]
> Hitting OpenSSL link errors? Build without the bundled OpenSSL and let Cargo use the system TLS stack (rustls):
> ```sh
> cargo build --release --no-default-features
> ```

**Install locally from source**

```sh
cargo install --path . --locked
```

---

## GitLab setup

labtui reads your repo's git remote URL and auto-detects whether it points to a GitLab instance.

On first launch inside a GitLab repo you'll be prompted for a **Personal Access Token**:

| Scope | Unlocks |
|---|---|
| `read_api` | Read-only browsing — MRs, Issues, CI logs |
| `api` | Write actions — merge, approve, create issue, comment, retry pipeline |

The token is stored in the **OS keyring** — never a plain-text file. You can also pass it via environment variable:

```sh
export GITLAB_TOKEN=your_token
```

---

## Usage

```sh
labtui
```

Launch inside any git repository. Navigate tabs with `Tab` / `Shift+Tab` or the number keys `1`–`8`.

| Key | Action |
|---|---|
| `Tab` / `Shift+Tab` | Next / previous tab |
| `1`–`5` | Git tabs — Status, Log, Files, Stash, Branches |
| `6` | Merge Requests |
| `7` | Issues |
| `8` | CI/CD |
| `?` | Toggle context-sensitive help |
| `q` | Quit |

## Key bindings

<details>
<summary><strong>Merge Requests (<code>6</code>)</strong></summary>

| Key | Action |
|---|---|
| `Enter` | Open MR detail + discussion thread |
| `d` | View the diff / changed files |
| `m` | Merge |
| `a` / `u` | Approve / unapprove |
| `b` | Rebase |
| `c` | Close / reopen |
| `n` | Add a comment |
| `l` | Edit labels |
| `o` | Open in browser |
| `f` | Filter (title, branch, author, label) |
| `r` | Refresh |

</details>

<details>
<summary><strong>Issues (<code>7</code>)</strong></summary>

| Key | Action |
|---|---|
| `Enter` | Open issue detail + discussion thread |
| `n` (list) | Create a new issue |
| `c` | Close / reopen |
| `l` | Edit labels |
| `o` | Open in browser |
| `b` | Toggle board view |
| `[` / `]` | Switch board |
| `←` / `→` | Move between board columns |
| `f` | Filter |
| `r` | Refresh |

</details>

<details>
<summary><strong>CI/CD (<code>8</code>)</strong></summary>

| Key | Action |
|---|---|
| `Enter` | Drill down — pipelines → jobs → job trace |
| `Esc` | Go back up one level |
| `t` | Retry pipeline or job |
| `x` | Cancel pipeline or job |
| `p` | Run a new pipeline |
| `c` | Toggle commits view |
| `o` | Open in browser |
| `r` | Refresh |

</details>

All key bindings are customizable via a Ron config file — see [KEY_CONFIG.md](KEY_CONFIG.md) for the full reference, including ready-made vim-style bindings.

---

## Color themes

labtui works on both light and dark terminals and ships with several built-in themes. See [THEMES.md](THEMES.md) for how to switch themes and write your own.

---

## GitLab coverage

See [`asyncgitlab/ROADMAP.md`](asyncgitlab/ROADMAP.md) for the complete coverage matrix (library vs. UI) and the list of upcoming features.

---

## License

[MIT](LICENSE.md) — labtui is a fork of [gitui](https://github.com/gitui-org/gitui), built on the shoulders of the project it started from.

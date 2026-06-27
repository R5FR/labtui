# GitLab coverage roadmap

The goal is to cover the GitLab features that are useful inside a TUI git
client — not the entire REST API (which has hundreds of endpoints: epics,
packages, registry, snippets, wiki, admin runners, …). This file tracks what
is implemented at the **library** level (`asyncgitlab`) versus what is wired
into the **UI** (`labtui`).

Legend: ✅ done · 🟡 library only (no UI yet) · ⬜ not started

## Foundation

| Capability | Library | UI |
|---|---|---|
| Remote URL parsing (ssh/https/self-hosted) | ✅ | ✅ |
| Token resolution (env + OS keyring) | ✅ | ✅ |
| Async job pattern on gitui threadpool | ✅ | ✅ |
| GET / POST / PUT / DELETE helpers | ✅ | — |
| Automatic pagination (`X-Next-Page`) | ✅ | — |
| Generic write-action job (`GitLabAction`) | ✅ | partial |

## Merge requests

| Capability | Library | UI |
|---|---|---|
| List (opened / all) | ✅ | ✅ |
| Filter (title/branch/author/label) | — | ✅ (`f`) |
| Get one | ✅ | ✅ |
| Create | ✅ | 🟡 |
| Merge | ✅ | ✅ (`m`) |
| Close / reopen | ✅ | ✅ (`c`) |
| Approve / unapprove | ✅ | ✅ (`a`/`u`) |
| Rebase | ✅ | ✅ (`b`) |
| Notes: list / add | ✅ | ✅ |
| Label editing | ✅ | ✅ (`l`) |
| CI / head pipeline badge | ✅ | ✅ (list + detail) |
| Detail view + discussion thread | ✅ | ✅ (`enter`) |
| Open in browser | ✅ | ✅ (`o`) |
| Diff / changes view | ✅ | ✅ (`d`) |

## Issues (priority)

| Capability | Library | UI |
|---|---|---|
| List (opened / all) | ✅ | ✅ |
| Filter (title/author/label) | — | ✅ (`f`) |
| Board view (columns by label, switchable) | ✅ | ✅ (`b`, `[`/`]`) |
| Get one | ✅ | ✅ |
| Create | ✅ | ✅ (`n`) |
| Close / reopen | ✅ | ✅ (`c`) |
| Notes: list / add | ✅ | ✅ |
| Label editing | ✅ | ✅ (`l`) |
| Detail view + discussion thread | ✅ | ✅ (`enter`) |
| Open in browser | ✅ | ✅ (`o`) |
| Assignees / milestone editing | ⬜ | ⬜ |

## Pipelines & CI/CD

| Capability | Library | UI |
|---|---|---|
| Latest pipeline (CI badge) | ✅ | ✅ (on MRs) |
| List pipelines | ✅ | ✅ (CI tab) |
| Pipeline jobs | ✅ | ✅ (`enter`) |
| Job trace (logs) | ✅ | ✅ (`enter`) |
| Create pipeline | ✅ | ✅ (`p`) |
| Retry / cancel pipeline | ✅ | ✅ (`t`/`x`) |
| Retry / cancel job | ✅ | ✅ (`t`/`x`) |
| Open in browser | ✅ | ✅ (`o`) |

## Repository

| Capability | Library | UI |
|---|---|---|
| Branches | ✅ | ⬜ (local git tab exists) |
| Tags | ✅ | ⬜ (local git tab exists) |
| Commits (with CI status) | ✅ | ✅ (CI tab, `c`) |
| Commit statuses | ✅ | ✅ (`enter` on a commit) |

## Not yet started (library or UI)

- Issue/MR assignee & milestone editing (needs member/milestone pickers)
- Server-side filtering & saved filters (current filter is client-side)
- Syntax-highlighted MR diff (currently raw unified diff)
- Branches / tags dedicated GitLab UI (local-git tabs already cover most needs)
- Members, project metadata / settings, environments, deployments
- Releases, packages, container registry, snippets, wiki

## Suggested next steps

1. **Assignee / milestone editing** — add member & milestone pickers, then
   wire `assignee_ids` / `milestone_id` updates.
2. **Syntax highlighting** in the MR diff view (reuse gitui's diff renderer).
3. **Server-side filters** for very large projects.

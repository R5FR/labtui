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
| Get one | ✅ | ✅ |
| Create | ✅ | 🟡 |
| Merge | ✅ | ✅ (`m`) |
| Close / reopen | ✅ | ✅ (`c`) |
| Approve / unapprove | ✅ | ✅ (`a`/`u`) |
| Rebase | ✅ | ✅ (`b`) |
| Notes: list / add | ✅ | ✅ |
| Detail view + discussion thread | ✅ | ✅ (`enter`) |
| Open in browser | ✅ | ✅ (`o`) |
| Diff / changes view | ⬜ | ⬜ |

## Issues (priority)

| Capability | Library | UI |
|---|---|---|
| List (opened / all) | ✅ | ✅ |
| Board view (columns by label, switchable) | ✅ | ✅ (`b`, `[`/`]`) |
| Get one | ✅ | ✅ |
| Create | ✅ | ✅ (`n`) |
| Close / reopen | ✅ | ✅ (`c`) |
| Notes: list / add | ✅ | ✅ |
| Detail view + discussion thread | ✅ | ✅ (`enter`) |
| Open in browser | ✅ | ✅ (`o`) |
| Assignees / labels / milestone editing | ⬜ | ⬜ |

## Pipelines & CI/CD

| Capability | Library | UI |
|---|---|---|
| Latest pipeline (CI badge) | ✅ | ⬜ |
| List pipelines | ✅ | ✅ (CI tab) |
| Pipeline jobs | ✅ | ✅ (`enter`) |
| Job trace (logs) | ✅ | ✅ (`enter`) |
| Create / retry / cancel pipeline | ✅ | ✅ retry/cancel (`t`/`x`) |
| Retry / cancel job | ✅ | ✅ (`t`/`x`) |
| Open in browser | ✅ | ✅ (`o`) |

## Not yet started (library or UI)

- MR diff / changes view
- Issue/MR assignees, labels, milestone editing; list filtering
- Pipeline filtering by current branch; trigger a new pipeline from the UI
- Branches / tags / commits via API, commit statuses
- Members, labels, milestones (for filtering & assignment)
- Project metadata / settings, environments, deployments
- Releases, packages, container registry, snippets, wiki

## Suggested next steps

1. **MR diff view** — show the changed files / diff of a merge request.
2. **Filtering** — filter issues/MRs by label, assignee, milestone; filter
   pipelines by the current branch.
3. **Issue/MR editing** — assignees, labels, milestone.

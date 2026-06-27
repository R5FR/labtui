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
| Get one | ✅ | ⬜ |
| Create | ✅ | 🟡 |
| Merge | ✅ | 🟡 |
| Close / reopen | ✅ | 🟡 |
| Approve / unapprove | ✅ | 🟡 |
| Rebase | ✅ | 🟡 |
| Notes: list / add | ✅ | ⬜ |
| Detail view + discussion thread | ⬜ | ⬜ |
| Diff / changes view | ⬜ | ⬜ |

## Issues (priority)

| Capability | Library | UI |
|---|---|---|
| List (opened / all) | ✅ | ✅ |
| Board view (columns by label) | ✅ | ✅ (`b`) |
| Get one | ✅ | ⬜ |
| Create | ✅ | ✅ (`n`) |
| Close / reopen | ✅ | ✅ close (`c`) |
| Notes: list / add | ✅ | ⬜ |
| Detail view + discussion thread | ⬜ | ⬜ |
| Assignees / labels / milestone editing | ⬜ | ⬜ |

## Pipelines & CI/CD

| Capability | Library | UI |
|---|---|---|
| Latest pipeline (CI badge) | ✅ | ⬜ |
| List pipelines | ✅ | ⬜ |
| Pipeline jobs | ✅ | ⬜ |
| Job trace (logs) | ✅ | ⬜ |
| Create / retry / cancel pipeline | ✅ | 🟡 |
| Retry / cancel job | ✅ | ⬜ |

## Not yet started (library or UI)

- Branches / tags / commits via API, commit statuses
- Members, labels, milestones (for filtering & assignment)
- "Open in browser" actions (we already have `web_url` on every object)
- Project metadata / settings, environments, deployments
- Releases, packages, container registry, snippets, wiki

## Suggested next steps

1. **MR & Issue detail views** — reuse the list tabs; on `Enter`, fetch the
   single object + notes and show a scrollable thread, with comment input.
2. **MR action keybindings** — surface the already-implemented merge/approve/
   close/rebase actions in `MergeRequestsTab` (same `AsyncActionJob` plumbing
   the Issues tab uses).
3. **Pipelines tab** — list pipelines for the current branch, drill into jobs,
   stream/show a job trace.
4. **"Open in browser"** — a single keybinding using `web_url`.

# asyncgitlab

Async GitLab API layer for **labtui**.

Provides:

- `remote` — parse a git remote URL into GitLab host + project path (ssh / https / self-hosted)
- `config` — resolve an access token (`GITLAB_TOKEN` or per-host `GITLAB_TOKEN_<HOST>`)
- `client` — async REST client (`reqwest` + `rustls`): GET/POST/PUT/DELETE helpers and
  automatic pagination (follows `X-Next-Page`)
- `job` — `AsyncJob` wrappers so GitLab calls run on labtui's threadpool
- `types` — trimmed serde deserialization targets

## API coverage

The `client` exposes:

- **Merge requests** — list, get, create, merge, close/reopen, approve/unapprove,
  rebase, list notes, add note
- **Issues** — list, get, create, close/reopen, list notes, add note
- **Pipelines & jobs** — list pipelines, latest pipeline, pipeline jobs, job trace,
  create/retry/cancel pipeline, retry/cancel job, delete pipeline

Write actions are expressed as a single [`GitLabAction`] enum executed by
[`AsyncActionJob`], so the UI never touches `async` directly.

TLS is pure-Rust (rustls); this crate does not link OpenSSL.

See [`ROADMAP.md`](ROADMAP.md) for what is wired into the UI and what is still
library-only.

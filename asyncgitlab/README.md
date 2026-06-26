# asyncgitlab

Async GitLab API layer for **labtui** (a [gitui](https://github.com/gitui-org/gitui) fork).

Provides:

- `remote` — parse a git remote URL into GitLab host + project path (ssh / https / self-hosted)
- `config` — resolve an access token (`GITLAB_TOKEN` or per-host `GITLAB_TOKEN_<HOST>`)
- `client` — async REST client (`reqwest` + `rustls`)
- `job` — `AsyncJob` wrappers so GitLab calls run on gitui's threadpool
- `types` — trimmed serde deserialization targets

TLS is pure-Rust (rustls); this crate does not link OpenSSL.

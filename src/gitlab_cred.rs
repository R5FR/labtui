//! Reuse the GitLab API token for git over HTTPS.
//!
//! GitLab rejects account passwords for git over HTTPS; it wants an access
//! token, sent as `oauth2:<token>`. When the git credential helper has
//! nothing for the remote but labtui already holds a token for that GitLab
//! host, use it instead of prompting.

use asyncgit::sync::{
	cred::BasicAuthCredential, get_remote_url, RepoPath,
};
use asyncgitlab::GitLabRemote;

/// Complete `cred` with the stored GitLab token for `remote`'s host, if
/// `cred` is incomplete and such a token exists.
pub fn fill_with_gitlab_token(
	repo: &RepoPath,
	remote: &str,
	cred: BasicAuthCredential,
) -> BasicAuthCredential {
	if cred.is_complete() {
		return cred;
	}

	let token = get_remote_url(repo, remote)
		.ok()
		.flatten()
		.filter(|url| url.starts_with("http"))
		.and_then(|url| GitLabRemote::from_url(&url).ok())
		.and_then(|gl| {
			asyncgitlab::config::resolve_token(&gl.host).ok()
		});

	token.map_or(cred, |token| {
		BasicAuthCredential::new(
			Some("oauth2".to_string()),
			Some(token),
		)
	})
}

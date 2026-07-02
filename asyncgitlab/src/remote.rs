//! Detection of the GitLab instance + project path from a git remote URL.
//!
//! Supports the URL shapes git uses in practice:
//!   - scp-like ssh:      `git@gitlab.com:group/sub/project.git`
//!   - full ssh:          `ssh://git@gitlab.example.com:2222/group/project.git`
//!   - https:             `https://gitlab.com/group/project.git`
//!   - https with creds:  `https://oauth2:TOKEN@gitlab.com/group/project.git`
//!
//! Works for self-hosted instances too: the host is taken from the URL, not
//! hard-coded to gitlab.com.

use crate::error::{Error, Result};

/// A GitLab project located on some instance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitLabRemote {
	/// e.g. `gitlab.com` or `gitlab.example.com`
	pub host: String,
	/// e.g. `group/subgroup/project` (no trailing `.git`)
	pub project_path: String,
}

impl GitLabRemote {
	/// Base REST endpoint, e.g. `https://gitlab.com/api/v4`.
	pub fn api_base(&self) -> String {
		format!("https://{}/api/v4", self.host)
	}

	/// URL-encoded project id usable in API paths, e.g. `group%2Fproject`.
	pub fn encoded_path(&self) -> String {
		// only `/` needs escaping for the namespaced-path form GitLab accepts;
		// path segments themselves are already URL-safe for git project names.
		self.project_path.replace('/', "%2F")
	}

	/// Web URL of the project, for "open in browser" actions.
	pub fn web_url(&self) -> String {
		format!("https://{}/{}", self.host, self.project_path)
	}

	/// Parse a git remote URL into a `GitLabRemote`.
	///
	/// Returns `UnsupportedRemote` if the URL cannot be understood; the caller
	/// decides whether that means "not a GitLab repo, hide the feature".
	pub fn from_url(remote: &str) -> Result<Self> {
		let unsupported =
			|| Error::UnsupportedRemote(remote.to_string());
		let url = remote.trim();

		let (host, path) = if let Some(rest) =
			url.strip_prefix("ssh://")
		{
			split_authority(rest).ok_or_else(unsupported)?
		} else if let Some(rest) = url.strip_prefix("https://") {
			split_authority(rest).ok_or_else(unsupported)?
		} else if let Some(rest) = url.strip_prefix("http://") {
			split_authority(rest).ok_or_else(unsupported)?
		} else if url.contains('@') && url.contains(':') {
			// scp-like: [user@]host:path
			let after_user =
				url.rsplit_once('@').map_or(url, |(_, h)| h);
			let (host, path) =
				after_user.split_once(':').ok_or_else(unsupported)?;
			(host.to_string(), path.to_string())
		} else {
			return Err(unsupported());
		};

		let host = strip_port(&host);
		let project_path = normalize_path(&path);

		if host.is_empty() || project_path.is_empty() {
			return Err(unsupported());
		}

		Ok(Self { host, project_path })
	}
}

/// Split `[user@]host[:port]/path` (the part after a scheme) into (host, path).
fn split_authority(rest: &str) -> Option<(String, String)> {
	// drop any `user:pass@` / `user@` credentials prefix
	let rest = rest.rsplit_once('@').map_or(rest, |(_, h)| h);
	let (authority, path) = rest.split_once('/')?;
	if authority.is_empty() || path.is_empty() {
		return None;
	}
	Some((authority.to_string(), path.to_string()))
}

/// Remove a trailing `:port` from a host authority.
fn strip_port(host: &str) -> String {
	host.split_once(':')
		.map_or(host, |(h, _)| h)
		.to_string()
}

/// Trim leading slashes and a trailing `.git`.
fn normalize_path(path: &str) -> String {
	path.trim_start_matches('/')
		.trim_end_matches('/')
		.strip_suffix(".git")
		.unwrap_or_else(|| path.trim_start_matches('/').trim_end_matches('/'))
		.to_string()
}

#[cfg(test)]
mod tests {
	use super::*;

	fn parse(u: &str) -> GitLabRemote {
		GitLabRemote::from_url(u).expect("should parse")
	}

	#[test]
	fn scp_like_ssh() {
		let r = parse("git@gitlab.com:group/project.git");
		assert_eq!(r.host, "gitlab.com");
		assert_eq!(r.project_path, "group/project");
	}

	#[test]
	fn scp_like_nested_groups() {
		let r = parse("git@gitlab.example.com:grp/sub/project.git");
		assert_eq!(r.host, "gitlab.example.com");
		assert_eq!(r.project_path, "grp/sub/project");
	}

	#[test]
	fn full_ssh_with_port() {
		let r = parse("ssh://git@gitlab.example.com:2222/group/project.git");
		assert_eq!(r.host, "gitlab.example.com");
		assert_eq!(r.project_path, "group/project");
	}

	#[test]
	fn https_plain() {
		let r = parse("https://gitlab.com/group/project.git");
		assert_eq!(r.host, "gitlab.com");
		assert_eq!(r.project_path, "group/project");
	}

	#[test]
	fn https_with_credentials() {
		let r = parse("https://oauth2:tok@gitlab.com/group/project.git");
		assert_eq!(r.host, "gitlab.com");
		assert_eq!(r.project_path, "group/project");
	}

	#[test]
	fn https_without_dot_git() {
		let r = parse("https://gitlab.com/group/project");
		assert_eq!(r.project_path, "group/project");
	}

	#[test]
	fn encoding_and_urls() {
		let r = parse("git@gitlab.com:grp/sub/project.git");
		assert_eq!(r.encoded_path(), "grp%2Fsub%2Fproject");
		assert_eq!(r.api_base(), "https://gitlab.com/api/v4");
		assert_eq!(r.web_url(), "https://gitlab.com/grp/sub/project");
	}

	#[test]
	fn rejects_non_gitlab_garbage() {
		assert!(GitLabRemote::from_url("not a url").is_err());
		assert!(GitLabRemote::from_url("https://gitlab.com/").is_err());
	}
}

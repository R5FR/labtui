//! Async REST client for a single GitLab project.

use crate::{
	error::{Error, Result},
	remote::GitLabRemote,
	types::{MergeRequest, Pipeline},
};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};

/// Which merge requests to fetch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MergeRequestScope {
	/// `state=opened`
	Opened,
	/// MRs whose source branch is the given one (set via `source_branch`).
	All,
}

/// A configured client bound to one project on one instance.
#[derive(Clone)]
pub struct GitLabClient {
	remote: GitLabRemote,
	http: reqwest::Client,
}

impl GitLabClient {
	/// Build a client from a parsed remote and a token.
	pub fn new(remote: GitLabRemote, token: &str) -> Result<Self> {
		let mut headers = HeaderMap::new();
		let mut auth = HeaderValue::from_str(&format!("Bearer {token}"))
			.map_err(|_| Error::MissingToken)?;
		auth.set_sensitive(true);
		headers.insert(AUTHORIZATION, auth);

		let http = reqwest::Client::builder()
			.user_agent(concat!("labtui/", env!("CARGO_PKG_VERSION")))
			.default_headers(headers)
			.build()?;

		Ok(Self { remote, http })
	}

	/// Convenience: detect token from the environment for the remote's host.
	pub fn from_env(remote: GitLabRemote) -> Result<Self> {
		let token = crate::config::resolve_token(&remote.host)?;
		Self::new(remote, &token)
	}

	pub fn remote(&self) -> &GitLabRemote {
		&self.remote
	}

	fn project_url(&self, suffix: &str) -> String {
		format!(
			"{}/projects/{}{}",
			self.remote.api_base(),
			self.remote.encoded_path(),
			suffix
		)
	}

	async fn get_json<T: serde::de::DeserializeOwned>(
		&self,
		url: &str,
	) -> Result<T> {
		let resp = self.http.get(url).send().await?;
		let status = resp.status();
		if !status.is_success() {
			let body = resp.text().await.unwrap_or_default();
			return Err(Error::Api { status: status.as_u16(), body });
		}
		Ok(resp.json::<T>().await?)
	}

	/// List merge requests for the project.
	pub async fn merge_requests(
		&self,
		scope: MergeRequestScope,
	) -> Result<Vec<MergeRequest>> {
		let query = match scope {
			MergeRequestScope::Opened => "?state=opened&per_page=50",
			MergeRequestScope::All => "?per_page=50",
		};
		self.get_json(&self.project_url(&format!(
			"/merge_requests{query}"
		)))
		.await
	}

	/// Latest pipeline for a given git ref (branch or sha), for the CI badge.
	pub async fn latest_pipeline(
		&self,
		git_ref: &str,
	) -> Result<Option<Pipeline>> {
		let url = self.project_url(&format!(
			"/pipelines?ref={git_ref}&per_page=1&order_by=id&sort=desc"
		));
		let list: Vec<Pipeline> = self.get_json(&url).await?;
		Ok(list.into_iter().next())
	}
}

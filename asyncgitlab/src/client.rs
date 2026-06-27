//! Async REST client for a single GitLab project.

use crate::{
	error::{Error, Result},
	remote::GitLabRemote,
	types::{Board, Issue, Job, MergeRequest, Note, Pipeline},
};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use serde_json::{json, Value};

/// Which merge requests to fetch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MergeRequestScope {
	/// `state=opened`
	Opened,
	/// every state
	All,
}

/// Which issues to fetch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueScope {
	/// `state=opened`
	Opened,
	/// every state
	All,
}

/// A `state_event` accepted by the issue/MR update endpoints.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateEvent {
	Close,
	Reopen,
}

impl StateEvent {
	fn as_str(self) -> &'static str {
		match self {
			StateEvent::Close => "close",
			StateEvent::Reopen => "reopen",
		}
	}
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

	// ---- low-level HTTP helpers -------------------------------------------

	/// Turn a response into the deserialized body, mapping non-2xx to an error.
	async fn parse<T: serde::de::DeserializeOwned>(
		resp: reqwest::Response,
	) -> Result<T> {
		let status = resp.status();
		if !status.is_success() {
			let body = resp.text().await.unwrap_or_default();
			return Err(Error::Api { status: status.as_u16(), body });
		}
		Ok(resp.json::<T>().await?)
	}

	/// Ensure the response was a success, discarding its body.
	async fn check(resp: reqwest::Response) -> Result<()> {
		let status = resp.status();
		if !status.is_success() {
			let body = resp.text().await.unwrap_or_default();
			return Err(Error::Api { status: status.as_u16(), body });
		}
		Ok(())
	}

	async fn get_json<T: serde::de::DeserializeOwned>(
		&self,
		url: &str,
	) -> Result<T> {
		Self::parse(self.http.get(url).send().await?).await
	}

	/// Fetch every page of a list endpoint, following `X-Next-Page`.
	async fn get_paginated<T: serde::de::DeserializeOwned>(
		&self,
		base_url: &str,
	) -> Result<Vec<T>> {
		let mut out: Vec<T> = Vec::new();
		let mut page = 1u32;
		loop {
			let sep = if base_url.contains('?') { '&' } else { '?' };
			let url =
				format!("{base_url}{sep}per_page=100&page={page}");
			let resp = self.http.get(&url).send().await?;
			let status = resp.status();
			if !status.is_success() {
				let body = resp.text().await.unwrap_or_default();
				return Err(Error::Api {
					status: status.as_u16(),
					body,
				});
			}
			let next = resp
				.headers()
				.get("x-next-page")
				.and_then(|v| v.to_str().ok())
				.and_then(|s| s.trim().parse::<u32>().ok());
			let mut chunk: Vec<T> = resp.json().await?;
			out.append(&mut chunk);
			match next {
				Some(n) if n > 0 => page = n,
				_ => break,
			}
		}
		Ok(out)
	}

	async fn post_json<T: serde::de::DeserializeOwned>(
		&self,
		url: &str,
		body: &Value,
	) -> Result<T> {
		Self::parse(self.http.post(url).json(body).send().await?).await
	}

	async fn put_json<T: serde::de::DeserializeOwned>(
		&self,
		url: &str,
		body: &Value,
	) -> Result<T> {
		Self::parse(self.http.put(url).json(body).send().await?).await
	}

	async fn delete(&self, url: &str) -> Result<()> {
		Self::check(self.http.delete(url).send().await?).await
	}

	// ---- merge requests ---------------------------------------------------

	/// List merge requests for the project.
	pub async fn merge_requests(
		&self,
		scope: MergeRequestScope,
	) -> Result<Vec<MergeRequest>> {
		let query = match scope {
			MergeRequestScope::Opened => "/merge_requests?state=opened",
			MergeRequestScope::All => "/merge_requests",
		};
		self.get_paginated(&self.project_url(query)).await
	}

	/// A single merge request by its project-scoped iid.
	pub async fn merge_request(
		&self,
		iid: u64,
	) -> Result<MergeRequest> {
		self.get_json(&self.project_url(&format!(
			"/merge_requests/{iid}"
		)))
		.await
	}

	/// Create a merge request from `source_branch` into `target_branch`.
	pub async fn create_merge_request(
		&self,
		source_branch: &str,
		target_branch: &str,
		title: &str,
	) -> Result<MergeRequest> {
		let body = json!({
			"source_branch": source_branch,
			"target_branch": target_branch,
			"title": title,
		});
		self.post_json(&self.project_url("/merge_requests"), &body)
			.await
	}

	/// Accept (merge) a merge request.
	pub async fn merge_merge_request(
		&self,
		iid: u64,
	) -> Result<MergeRequest> {
		self.put_json(
			&self.project_url(&format!(
				"/merge_requests/{iid}/merge"
			)),
			&json!({}),
		)
		.await
	}

	/// Close or reopen a merge request.
	pub async fn set_merge_request_state(
		&self,
		iid: u64,
		event: StateEvent,
	) -> Result<MergeRequest> {
		let body = json!({ "state_event": event.as_str() });
		self.put_json(
			&self.project_url(&format!("/merge_requests/{iid}")),
			&body,
		)
		.await
	}

	/// Approve a merge request.
	pub async fn approve_merge_request(
		&self,
		iid: u64,
	) -> Result<()> {
		Self::check(
			self.http
				.post(self.project_url(&format!(
					"/merge_requests/{iid}/approve"
				)))
				.send()
				.await?,
		)
		.await
	}

	/// Remove the caller's approval from a merge request.
	pub async fn unapprove_merge_request(
		&self,
		iid: u64,
	) -> Result<()> {
		Self::check(
			self.http
				.post(self.project_url(&format!(
					"/merge_requests/{iid}/unapprove"
				)))
				.send()
				.await?,
		)
		.await
	}

	/// Trigger a rebase of a merge request onto its target branch.
	pub async fn rebase_merge_request(
		&self,
		iid: u64,
	) -> Result<()> {
		Self::check(
			self.http
				.put(self.project_url(&format!(
					"/merge_requests/{iid}/rebase"
				)))
				.send()
				.await?,
		)
		.await
	}

	/// Notes (comments) on a merge request, oldest first.
	pub async fn merge_request_notes(
		&self,
		iid: u64,
	) -> Result<Vec<Note>> {
		self.get_paginated(&self.project_url(&format!(
			"/merge_requests/{iid}/notes?sort=asc&order_by=created_at"
		)))
		.await
	}

	/// Add a note (comment) to a merge request.
	pub async fn create_merge_request_note(
		&self,
		iid: u64,
		body: &str,
	) -> Result<Note> {
		self.post_json(
			&self.project_url(&format!(
				"/merge_requests/{iid}/notes"
			)),
			&json!({ "body": body }),
		)
		.await
	}

	// ---- issues -----------------------------------------------------------

	/// List issues for the project.
	pub async fn issues(
		&self,
		scope: IssueScope,
	) -> Result<Vec<Issue>> {
		let query = match scope {
			IssueScope::Opened => "/issues?state=opened",
			IssueScope::All => "/issues",
		};
		self.get_paginated(&self.project_url(query)).await
	}

	/// A single issue by its project-scoped iid.
	pub async fn issue(&self, iid: u64) -> Result<Issue> {
		self.get_json(&self.project_url(&format!("/issues/{iid}")))
			.await
	}

	/// Create an issue.
	pub async fn create_issue(
		&self,
		title: &str,
		description: Option<&str>,
	) -> Result<Issue> {
		let mut body = json!({ "title": title });
		if let Some(desc) = description {
			body["description"] = json!(desc);
		}
		self.post_json(&self.project_url("/issues"), &body).await
	}

	/// Close or reopen an issue.
	pub async fn set_issue_state(
		&self,
		iid: u64,
		event: StateEvent,
	) -> Result<Issue> {
		let body = json!({ "state_event": event.as_str() });
		self.put_json(
			&self.project_url(&format!("/issues/{iid}")),
			&body,
		)
		.await
	}

	/// Notes (comments) on an issue, oldest first.
	pub async fn issue_notes(
		&self,
		iid: u64,
	) -> Result<Vec<Note>> {
		self.get_paginated(&self.project_url(&format!(
			"/issues/{iid}/notes?sort=asc&order_by=created_at"
		)))
		.await
	}

	/// Add a note (comment) to an issue.
	pub async fn create_issue_note(
		&self,
		iid: u64,
		body: &str,
	) -> Result<Note> {
		self.post_json(
			&self.project_url(&format!("/issues/{iid}/notes")),
			&json!({ "body": body }),
		)
		.await
	}

	/// Issue boards configured for the project.
	pub async fn boards(&self) -> Result<Vec<Board>> {
		self.get_paginated(&self.project_url("/boards")).await
	}

	// ---- pipelines & jobs -------------------------------------------------

	/// List pipelines, optionally filtered to a single git ref.
	pub async fn pipelines(
		&self,
		git_ref: Option<&str>,
	) -> Result<Vec<Pipeline>> {
		let suffix = match git_ref {
			Some(r) => {
				format!("/pipelines?ref={r}&order_by=id&sort=desc")
			}
			None => "/pipelines?order_by=id&sort=desc".to_string(),
		};
		self.get_paginated(&self.project_url(&suffix)).await
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

	/// Jobs belonging to a pipeline.
	pub async fn pipeline_jobs(
		&self,
		pipeline_id: u64,
	) -> Result<Vec<Job>> {
		self.get_paginated(&self.project_url(&format!(
			"/pipelines/{pipeline_id}/jobs"
		)))
		.await
	}

	/// Raw trace (log) of a job as plain text.
	pub async fn job_trace(&self, job_id: u64) -> Result<String> {
		let url =
			self.project_url(&format!("/jobs/{job_id}/trace"));
		let resp = self.http.get(&url).send().await?;
		let status = resp.status();
		if !status.is_success() {
			let body = resp.text().await.unwrap_or_default();
			return Err(Error::Api { status: status.as_u16(), body });
		}
		Ok(resp.text().await?)
	}

	/// Trigger a new pipeline for a git ref.
	pub async fn create_pipeline(
		&self,
		git_ref: &str,
	) -> Result<Pipeline> {
		self.post_json(
			&self.project_url("/pipeline"),
			&json!({ "ref": git_ref }),
		)
		.await
	}

	/// Retry the failed/canceled jobs of a pipeline.
	pub async fn retry_pipeline(
		&self,
		pipeline_id: u64,
	) -> Result<Pipeline> {
		self.post_json(
			&self.project_url(&format!(
				"/pipelines/{pipeline_id}/retry"
			)),
			&json!({}),
		)
		.await
	}

	/// Cancel a running pipeline.
	pub async fn cancel_pipeline(
		&self,
		pipeline_id: u64,
	) -> Result<Pipeline> {
		self.post_json(
			&self.project_url(&format!(
				"/pipelines/{pipeline_id}/cancel"
			)),
			&json!({}),
		)
		.await
	}

	/// Retry a single job.
	pub async fn retry_job(&self, job_id: u64) -> Result<Job> {
		self.post_json(
			&self.project_url(&format!("/jobs/{job_id}/retry")),
			&json!({}),
		)
		.await
	}

	/// Cancel a single job.
	pub async fn cancel_job(&self, job_id: u64) -> Result<Job> {
		self.post_json(
			&self.project_url(&format!("/jobs/{job_id}/cancel")),
			&json!({}),
		)
		.await
	}

	/// Delete a pipeline.
	pub async fn delete_pipeline(
		&self,
		pipeline_id: u64,
	) -> Result<()> {
		self.delete(&self.project_url(&format!(
			"/pipelines/{pipeline_id}"
		)))
		.await
	}
}

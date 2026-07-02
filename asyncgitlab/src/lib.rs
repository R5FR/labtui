//! asyncgitlab — the GitLab API layer for labtui.
//!
//! Phase 0/1 foundation:
//!   - [`remote`]  parse a git remote URL into host + project path
//!   - [`config`]  resolve an access token
//!   - [`client`]  async REST client (reqwest + rustls)
//!   - [`types`]   trimmed deserialization structs
//!
//! The client is plain `async`. Later it gets wrapped in labtui's `AsyncJob`
//! pattern so calls run off the UI thread and report back over a channel.

pub mod board;
pub mod client;
pub mod config;
pub mod error;
pub mod job;
pub mod remote;
pub mod runtime;
pub mod types;

pub use board::{build_board, BoardColumn, BoardView};
pub use client::{
	GitLabClient, IssueScope, MergeRequestScope, StateEvent,
};
pub use config::{
	delete_token, has_token, resolve_token, store_token,
};
pub use error::{Error, Result};
pub use job::{
	ActionResult, AsyncActionJob, AsyncBoardJob,
	AsyncCommitStatusesJob, AsyncCommitsJob, AsyncGitLabNotification,
	AsyncIssueDetailJob, AsyncIssuesJob, AsyncMergeRequestsJob,
	AsyncMrChangesJob, AsyncMrDetailJob, AsyncPipelineJobsJob,
	AsyncPipelinesJob, AsyncTraceJob, BoardResult,
	CommitStatusesResult, CommitsResult, GitLabAction,
	IssueDetailResult, IssuesResult, MergeRequestsResult,
	MrChangesResult, MrDetailResult, PipelineJobsResult,
	PipelinesResult, TraceResult,
};
pub use remote::GitLabRemote;
pub use types::{
	Board, BoardList, Branch, ChangedFile, CiStatus, Commit,
	CommitRef, CommitStatus, Issue, IssueState, Job, Label,
	MergeRequest, MergeRequestState, MrChanges, Note, Pipeline,
	PipelineStatus, Tag, User,
};

/// Build a client straight from a git remote URL, using a token from the
/// environment. Returns `Ok(None)` when the remote is not a GitLab URL, so the
/// caller can simply hide GitLab features for non-GitLab repos.
pub fn client_from_remote(
	remote_url: &str,
) -> Result<Option<GitLabClient>> {
	match GitLabRemote::from_url(remote_url) {
		Ok(remote) => Ok(Some(GitLabClient::from_env(remote)?)),
		Err(Error::UnsupportedRemote(_)) => Ok(None),
		Err(e) => Err(e),
	}
}

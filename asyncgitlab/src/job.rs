//! `AsyncJob` implementations so GitLab requests run on gitui's threadpool and
//! report completion over the same notification channel mechanism as git tasks.
//!
//! Pattern mirrors `asyncgit::AsyncFetchJob`: the job keeps its state behind an
//! `Arc<Mutex<..>>`, does the blocking work in `run`, stores the outcome, and
//! always returns `Ok(notification)`. The UI then calls `take_last()` on the
//! `AsyncSingleJob` and reads the result.

use crate::{
	board::{build_board, BoardColumn},
	client::{
		GitLabClient, IssueScope, MergeRequestScope, StateEvent,
	},
	error::Error,
	remote::GitLabRemote,
	runtime,
	types::{Issue, MergeRequest, Note},
};
use asyncgit::{
	asyncjob::{AsyncJob, RunParams},
	Result as GitResult,
};
use std::sync::{Arc, Mutex};

/// Copy notification telling the UI which GitLab job finished. The payload is
/// retrieved separately via the job's accessor (notifications must be `Copy`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AsyncGitLabNotification {
	/// merge request list finished loading
	MergeRequests,
	/// issue list finished loading
	Issues,
	/// issue board finished loading
	Board,
	/// a single issue's detail + notes finished loading
	IssueDetail,
	/// a write action (create/close/comment/merge/…) finished
	Action,
}

/// Result the UI reads after the merge-request job completes.
pub type MergeRequestsResult = Result<Vec<MergeRequest>, String>;

/// Result the UI reads after the issues job completes.
pub type IssuesResult = Result<Vec<Issue>, String>;

/// Result the UI reads after the board job completes.
pub type BoardResult = Result<Vec<BoardColumn>, String>;

/// Result the UI reads after the issue-detail job completes: the issue plus
/// its notes (comments), oldest first.
pub type IssueDetailResult = Result<(Issue, Vec<Note>), String>;

/// Result of a write action: a human-readable success message, or an error.
pub type ActionResult = Result<String, String>;

enum JobState {
	Request {
		remote: GitLabRemote,
		scope: MergeRequestScope,
	},
	Response(MergeRequestsResult),
}

/// Fetches merge requests for a project off the UI thread.
#[derive(Clone)]
pub struct AsyncMergeRequestsJob {
	state: Arc<Mutex<Option<JobState>>>,
}

impl AsyncMergeRequestsJob {
	pub fn new(
		remote: GitLabRemote,
		scope: MergeRequestScope,
	) -> Self {
		Self {
			state: Arc::new(Mutex::new(Some(JobState::Request {
				remote,
				scope,
			}))),
		}
	}

	/// Outcome of the job once finished; `None` while still pending.
	pub fn result(&self) -> Option<MergeRequestsResult> {
		let state = self.state.lock().ok()?;
		match state.as_ref()? {
			JobState::Response(r) => Some(r.clone()),
			JobState::Request { .. } => None,
		}
	}

	fn fetch(
		remote: &GitLabRemote,
		scope: MergeRequestScope,
	) -> Result<Vec<MergeRequest>, Error> {
		let client = GitLabClient::from_env(remote.clone())?;
		runtime::block_on(client.merge_requests(scope))
	}
}

impl AsyncJob for AsyncMergeRequestsJob {
	type Notification = AsyncGitLabNotification;
	type Progress = ();

	fn run(
		&mut self,
		_params: RunParams<Self::Notification, Self::Progress>,
	) -> GitResult<Self::Notification> {
		if let Ok(mut state) = self.state.lock() {
			*state = state.take().map(|state| match state {
				JobState::Request { remote, scope } => {
					let result = Self::fetch(&remote, scope)
						.map_err(|e| e.to_string());
					JobState::Response(result)
				}
				JobState::Response(r) => JobState::Response(r),
			});
		}

		Ok(AsyncGitLabNotification::MergeRequests)
	}
}

enum IssuesJobState {
	Request {
		remote: GitLabRemote,
		scope: IssueScope,
	},
	Response(IssuesResult),
}

/// Fetches issues for a project off the UI thread.
#[derive(Clone)]
pub struct AsyncIssuesJob {
	state: Arc<Mutex<Option<IssuesJobState>>>,
}

impl AsyncIssuesJob {
	pub fn new(remote: GitLabRemote, scope: IssueScope) -> Self {
		Self {
			state: Arc::new(Mutex::new(Some(
				IssuesJobState::Request { remote, scope },
			))),
		}
	}

	/// Outcome of the job once finished; `None` while still pending.
	pub fn result(&self) -> Option<IssuesResult> {
		let state = self.state.lock().ok()?;
		match state.as_ref()? {
			IssuesJobState::Response(r) => Some(r.clone()),
			IssuesJobState::Request { .. } => None,
		}
	}

	fn fetch(
		remote: &GitLabRemote,
		scope: IssueScope,
	) -> Result<Vec<Issue>, Error> {
		let client = GitLabClient::from_env(remote.clone())?;
		runtime::block_on(client.issues(scope))
	}
}

impl AsyncJob for AsyncIssuesJob {
	type Notification = AsyncGitLabNotification;
	type Progress = ();

	fn run(
		&mut self,
		_params: RunParams<Self::Notification, Self::Progress>,
	) -> GitResult<Self::Notification> {
		if let Ok(mut state) = self.state.lock() {
			*state = state.take().map(|state| match state {
				IssuesJobState::Request { remote, scope } => {
					let result = Self::fetch(&remote, scope)
						.map_err(|e| e.to_string());
					IssuesJobState::Response(result)
				}
				IssuesJobState::Response(r) => {
					IssuesJobState::Response(r)
				}
			});
		}

		Ok(AsyncGitLabNotification::Issues)
	}
}

enum BoardJobState {
	Request { remote: GitLabRemote },
	Response(BoardResult),
}

/// Fetches the project's first issue board and buckets all issues into its
/// columns, off the UI thread.
#[derive(Clone)]
pub struct AsyncBoardJob {
	state: Arc<Mutex<Option<BoardJobState>>>,
}

impl AsyncBoardJob {
	pub fn new(remote: GitLabRemote) -> Self {
		Self {
			state: Arc::new(Mutex::new(Some(
				BoardJobState::Request { remote },
			))),
		}
	}

	/// Outcome of the job once finished; `None` while still pending.
	pub fn result(&self) -> Option<BoardResult> {
		let state = self.state.lock().ok()?;
		match state.as_ref()? {
			BoardJobState::Response(r) => Some(r.clone()),
			BoardJobState::Request { .. } => None,
		}
	}

	fn fetch(
		remote: &GitLabRemote,
	) -> Result<Vec<BoardColumn>, Error> {
		let client = GitLabClient::from_env(remote.clone())?;
		runtime::block_on(async {
			let lists = client
				.boards()
				.await?
				.into_iter()
				.next()
				.map(|b| b.lists)
				.unwrap_or_default();
			let issues = client.issues(IssueScope::All).await?;
			Ok(build_board(&lists, issues))
		})
	}
}

impl AsyncJob for AsyncBoardJob {
	type Notification = AsyncGitLabNotification;
	type Progress = ();

	fn run(
		&mut self,
		_params: RunParams<Self::Notification, Self::Progress>,
	) -> GitResult<Self::Notification> {
		if let Ok(mut state) = self.state.lock() {
			*state = state.take().map(|state| match state {
				BoardJobState::Request { remote } => {
					let result = Self::fetch(&remote)
						.map_err(|e| e.to_string());
					BoardJobState::Response(result)
				}
				BoardJobState::Response(r) => {
					BoardJobState::Response(r)
				}
			});
		}

		Ok(AsyncGitLabNotification::Board)
	}
}

enum IssueDetailJobState {
	Request { remote: GitLabRemote, iid: u64 },
	Response(IssueDetailResult),
}

/// Fetches a single issue plus its notes, off the UI thread.
#[derive(Clone)]
pub struct AsyncIssueDetailJob {
	state: Arc<Mutex<Option<IssueDetailJobState>>>,
}

impl AsyncIssueDetailJob {
	pub fn new(remote: GitLabRemote, iid: u64) -> Self {
		Self {
			state: Arc::new(Mutex::new(Some(
				IssueDetailJobState::Request { remote, iid },
			))),
		}
	}

	/// Outcome of the job once finished; `None` while still pending.
	pub fn result(&self) -> Option<IssueDetailResult> {
		let state = self.state.lock().ok()?;
		match state.as_ref()? {
			IssueDetailJobState::Response(r) => Some(r.clone()),
			IssueDetailJobState::Request { .. } => None,
		}
	}

	fn fetch(
		remote: &GitLabRemote,
		iid: u64,
	) -> Result<(Issue, Vec<Note>), Error> {
		let client = GitLabClient::from_env(remote.clone())?;
		runtime::block_on(async {
			let issue = client.issue(iid).await?;
			let notes = client.issue_notes(iid).await?;
			Ok((issue, notes))
		})
	}
}

impl AsyncJob for AsyncIssueDetailJob {
	type Notification = AsyncGitLabNotification;
	type Progress = ();

	fn run(
		&mut self,
		_params: RunParams<Self::Notification, Self::Progress>,
	) -> GitResult<Self::Notification> {
		if let Ok(mut state) = self.state.lock() {
			*state = state.take().map(|state| match state {
				IssueDetailJobState::Request { remote, iid } => {
					let result = Self::fetch(&remote, iid)
						.map_err(|e| e.to_string());
					IssueDetailJobState::Response(result)
				}
				IssueDetailJobState::Response(r) => {
					IssueDetailJobState::Response(r)
				}
			});
		}

		Ok(AsyncGitLabNotification::IssueDetail)
	}
}

/// A one-shot write action against the GitLab API. Kept as a data enum (rather
/// than a closure) so all `async` work stays inside this crate's runtime.
#[derive(Debug, Clone)]
pub enum GitLabAction {
	// issues
	CreateIssue {
		title: String,
		description: Option<String>,
	},
	SetIssueState {
		iid: u64,
		event: StateEvent,
	},
	CreateIssueNote {
		iid: u64,
		body: String,
	},
	// merge requests
	CreateMergeRequest {
		source_branch: String,
		target_branch: String,
		title: String,
	},
	MergeMergeRequest {
		iid: u64,
	},
	SetMergeRequestState {
		iid: u64,
		event: StateEvent,
	},
	ApproveMergeRequest {
		iid: u64,
	},
	UnapproveMergeRequest {
		iid: u64,
	},
	RebaseMergeRequest {
		iid: u64,
	},
	CreateMergeRequestNote {
		iid: u64,
		body: String,
	},
	// pipelines
	CreatePipeline {
		git_ref: String,
	},
	RetryPipeline {
		id: u64,
	},
	CancelPipeline {
		id: u64,
	},
}

impl GitLabAction {
	async fn run(
		self,
		client: &GitLabClient,
	) -> Result<String, Error> {
		match self {
			GitLabAction::CreateIssue { title, description } => {
				let issue = client
					.create_issue(&title, description.as_deref())
					.await?;
				Ok(format!("issue #{} created", issue.iid))
			}
			GitLabAction::SetIssueState { iid, event } => {
				client.set_issue_state(iid, event).await?;
				Ok(format!("issue #{iid} updated"))
			}
			GitLabAction::CreateIssueNote { iid, body } => {
				client.create_issue_note(iid, &body).await?;
				Ok(format!("comment added to issue #{iid}"))
			}
			GitLabAction::CreateMergeRequest {
				source_branch,
				target_branch,
				title,
			} => {
				let mr = client
					.create_merge_request(
						&source_branch,
						&target_branch,
						&title,
					)
					.await?;
				Ok(format!("merge request !{} created", mr.iid))
			}
			GitLabAction::MergeMergeRequest { iid } => {
				client.merge_merge_request(iid).await?;
				Ok(format!("merge request !{iid} merged"))
			}
			GitLabAction::SetMergeRequestState { iid, event } => {
				client.set_merge_request_state(iid, event).await?;
				Ok(format!("merge request !{iid} updated"))
			}
			GitLabAction::ApproveMergeRequest { iid } => {
				client.approve_merge_request(iid).await?;
				Ok(format!("merge request !{iid} approved"))
			}
			GitLabAction::UnapproveMergeRequest { iid } => {
				client.unapprove_merge_request(iid).await?;
				Ok(format!("merge request !{iid} unapproved"))
			}
			GitLabAction::RebaseMergeRequest { iid } => {
				client.rebase_merge_request(iid).await?;
				Ok(format!("merge request !{iid} rebasing"))
			}
			GitLabAction::CreateMergeRequestNote { iid, body } => {
				client.create_merge_request_note(iid, &body).await?;
				Ok(format!("comment added to !{iid}"))
			}
			GitLabAction::CreatePipeline { git_ref } => {
				let p = client.create_pipeline(&git_ref).await?;
				Ok(format!("pipeline #{} started", p.id))
			}
			GitLabAction::RetryPipeline { id } => {
				client.retry_pipeline(id).await?;
				Ok(format!("pipeline #{id} retried"))
			}
			GitLabAction::CancelPipeline { id } => {
				client.cancel_pipeline(id).await?;
				Ok(format!("pipeline #{id} canceled"))
			}
		}
	}
}

enum ActionJobState {
	Request {
		remote: GitLabRemote,
		action: GitLabAction,
	},
	Response(ActionResult),
}

/// Runs a single write action off the UI thread.
#[derive(Clone)]
pub struct AsyncActionJob {
	state: Arc<Mutex<Option<ActionJobState>>>,
}

impl AsyncActionJob {
	pub fn new(remote: GitLabRemote, action: GitLabAction) -> Self {
		Self {
			state: Arc::new(Mutex::new(Some(
				ActionJobState::Request { remote, action },
			))),
		}
	}

	/// Outcome of the job once finished; `None` while still pending.
	pub fn result(&self) -> Option<ActionResult> {
		let state = self.state.lock().ok()?;
		match state.as_ref()? {
			ActionJobState::Response(r) => Some(r.clone()),
			ActionJobState::Request { .. } => None,
		}
	}

	fn perform(
		remote: GitLabRemote,
		action: GitLabAction,
	) -> Result<String, Error> {
		let client = GitLabClient::from_env(remote)?;
		runtime::block_on(action.run(&client))
	}
}

impl AsyncJob for AsyncActionJob {
	type Notification = AsyncGitLabNotification;
	type Progress = ();

	fn run(
		&mut self,
		_params: RunParams<Self::Notification, Self::Progress>,
	) -> GitResult<Self::Notification> {
		if let Ok(mut state) = self.state.lock() {
			*state = state.take().map(|state| match state {
				ActionJobState::Request { remote, action } => {
					let result = Self::perform(remote, action)
						.map_err(|e| e.to_string());
					ActionJobState::Response(result)
				}
				ActionJobState::Response(r) => {
					ActionJobState::Response(r)
				}
			});
		}

		Ok(AsyncGitLabNotification::Action)
	}
}

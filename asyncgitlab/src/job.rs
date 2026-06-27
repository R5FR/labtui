//! `AsyncJob` implementations so GitLab requests run on gitui's threadpool and
//! report completion over the same notification channel mechanism as git tasks.
//!
//! Pattern mirrors `asyncgit::AsyncFetchJob`: the job keeps its state behind an
//! `Arc<Mutex<..>>`, does the blocking work in `run`, stores the outcome, and
//! always returns `Ok(notification)`. The UI then calls `take_last()` on the
//! `AsyncSingleJob` and reads the result.

use crate::{
	board::{build_board, BoardView},
	client::{
		GitLabClient, IssueScope, MergeRequestScope, StateEvent,
	},
	error::Error,
	remote::GitLabRemote,
	runtime,
	types::{
		Commit, CommitStatus, Issue, Job, MergeRequest, MrChanges,
		Note, Pipeline,
	},
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
	/// a single merge request's detail + notes finished loading
	MrDetail,
	/// pipeline list finished loading
	Pipelines,
	/// a pipeline's job list finished loading
	PipelineJobs,
	/// a job's trace (log) finished loading
	JobTrace,
	/// a merge request's changes (diff) finished loading
	MrChanges,
	/// the commit list finished loading
	Commits,
	/// a commit's statuses finished loading
	CommitStatuses,
	/// a write action (create/close/comment/merge/…) finished
	Action,
}

/// Result the UI reads after the merge-request job completes.
pub type MergeRequestsResult = Result<Vec<MergeRequest>, String>;

/// Result the UI reads after the issues job completes.
pub type IssuesResult = Result<Vec<Issue>, String>;

/// Result the UI reads after the board job completes.
pub type BoardResult = Result<BoardView, String>;

/// Result the UI reads after the issue-detail job completes: the issue plus
/// its notes (comments), oldest first.
pub type IssueDetailResult = Result<(Issue, Vec<Note>), String>;

/// Result the UI reads after the MR-detail job completes: the merge request
/// plus its notes (comments), oldest first.
pub type MrDetailResult = Result<(MergeRequest, Vec<Note>), String>;

/// Result the UI reads after the pipelines job completes.
pub type PipelinesResult = Result<Vec<Pipeline>, String>;

/// Result the UI reads after the pipeline-jobs job completes.
pub type PipelineJobsResult = Result<Vec<Job>, String>;

/// Result the UI reads after the job-trace job completes.
pub type TraceResult = Result<String, String>;

/// Result the UI reads after the MR-changes job completes.
pub type MrChangesResult = Result<MrChanges, String>;

/// Result the UI reads after the commits job completes.
pub type CommitsResult = Result<Vec<Commit>, String>;

/// Result the UI reads after the commit-statuses job completes.
pub type CommitStatusesResult = Result<Vec<CommitStatus>, String>;

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
	Request { remote: GitLabRemote, index: usize },
	Response(BoardResult),
}

/// Fetches one of the project's issue boards (by index) and buckets all issues
/// into its columns, off the UI thread.
#[derive(Clone)]
pub struct AsyncBoardJob {
	state: Arc<Mutex<Option<BoardJobState>>>,
}

impl AsyncBoardJob {
	/// `index` selects which board to show; it is clamped to the number of
	/// boards the project actually has.
	pub fn new(remote: GitLabRemote, index: usize) -> Self {
		Self {
			state: Arc::new(Mutex::new(Some(
				BoardJobState::Request { remote, index },
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
		index: usize,
	) -> Result<BoardView, Error> {
		let client = GitLabClient::from_env(remote.clone())?;
		runtime::block_on(async {
			let boards = client.boards().await?;
			let board_names: Vec<String> = boards
				.iter()
				.map(|b| {
					if b.name.is_empty() {
						format!("board {}", b.id)
					} else {
						b.name.clone()
					}
				})
				.collect();
			let selected =
				index.min(boards.len().saturating_sub(1));
			let lists = boards
				.get(selected)
				.map(|b| b.lists.clone())
				.unwrap_or_default();
			let issues = client.issues(IssueScope::All).await?;
			Ok(BoardView {
				board_names,
				selected,
				columns: build_board(&lists, issues),
			})
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
				BoardJobState::Request { remote, index } => {
					let result = Self::fetch(&remote, index)
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

#[allow(clippy::large_enum_variant)]
enum MrDetailJobState {
	Request { remote: GitLabRemote, iid: u64 },
	Response(MrDetailResult),
}

/// Fetches a single merge request plus its notes, off the UI thread.
#[derive(Clone)]
pub struct AsyncMrDetailJob {
	state: Arc<Mutex<Option<MrDetailJobState>>>,
}

impl AsyncMrDetailJob {
	pub fn new(remote: GitLabRemote, iid: u64) -> Self {
		Self {
			state: Arc::new(Mutex::new(Some(
				MrDetailJobState::Request { remote, iid },
			))),
		}
	}

	/// Outcome of the job once finished; `None` while still pending.
	pub fn result(&self) -> Option<MrDetailResult> {
		let state = self.state.lock().ok()?;
		match state.as_ref()? {
			MrDetailJobState::Response(r) => Some(r.clone()),
			MrDetailJobState::Request { .. } => None,
		}
	}

	fn fetch(
		remote: &GitLabRemote,
		iid: u64,
	) -> Result<(MergeRequest, Vec<Note>), Error> {
		let client = GitLabClient::from_env(remote.clone())?;
		runtime::block_on(async {
			let mr = client.merge_request(iid).await?;
			let notes = client.merge_request_notes(iid).await?;
			Ok((mr, notes))
		})
	}
}

impl AsyncJob for AsyncMrDetailJob {
	type Notification = AsyncGitLabNotification;
	type Progress = ();

	fn run(
		&mut self,
		_params: RunParams<Self::Notification, Self::Progress>,
	) -> GitResult<Self::Notification> {
		if let Ok(mut state) = self.state.lock() {
			*state = state.take().map(|state| match state {
				MrDetailJobState::Request { remote, iid } => {
					let result = Self::fetch(&remote, iid)
						.map_err(|e| e.to_string());
					MrDetailJobState::Response(result)
				}
				MrDetailJobState::Response(r) => {
					MrDetailJobState::Response(r)
				}
			});
		}

		Ok(AsyncGitLabNotification::MrDetail)
	}
}

enum PipelinesJobState {
	Request { remote: GitLabRemote },
	Response(PipelinesResult),
}

/// Fetches the project's recent pipelines, off the UI thread.
#[derive(Clone)]
pub struct AsyncPipelinesJob {
	state: Arc<Mutex<Option<PipelinesJobState>>>,
}

impl AsyncPipelinesJob {
	pub fn new(remote: GitLabRemote) -> Self {
		Self {
			state: Arc::new(Mutex::new(Some(
				PipelinesJobState::Request { remote },
			))),
		}
	}

	pub fn result(&self) -> Option<PipelinesResult> {
		let state = self.state.lock().ok()?;
		match state.as_ref()? {
			PipelinesJobState::Response(r) => Some(r.clone()),
			PipelinesJobState::Request { .. } => None,
		}
	}

	fn fetch(
		remote: &GitLabRemote,
	) -> Result<Vec<Pipeline>, Error> {
		let client = GitLabClient::from_env(remote.clone())?;
		runtime::block_on(client.pipelines(None))
	}
}

impl AsyncJob for AsyncPipelinesJob {
	type Notification = AsyncGitLabNotification;
	type Progress = ();

	fn run(
		&mut self,
		_params: RunParams<Self::Notification, Self::Progress>,
	) -> GitResult<Self::Notification> {
		if let Ok(mut state) = self.state.lock() {
			*state = state.take().map(|state| match state {
				PipelinesJobState::Request { remote } => {
					PipelinesJobState::Response(
						Self::fetch(&remote)
							.map_err(|e| e.to_string()),
					)
				}
				PipelinesJobState::Response(r) => {
					PipelinesJobState::Response(r)
				}
			});
		}
		Ok(AsyncGitLabNotification::Pipelines)
	}
}

enum PipelineJobsJobState {
	Request { remote: GitLabRemote, pipeline_id: u64 },
	Response(PipelineJobsResult),
}

/// Fetches the jobs of one pipeline, off the UI thread.
#[derive(Clone)]
pub struct AsyncPipelineJobsJob {
	state: Arc<Mutex<Option<PipelineJobsJobState>>>,
}

impl AsyncPipelineJobsJob {
	pub fn new(remote: GitLabRemote, pipeline_id: u64) -> Self {
		Self {
			state: Arc::new(Mutex::new(Some(
				PipelineJobsJobState::Request {
					remote,
					pipeline_id,
				},
			))),
		}
	}

	pub fn result(&self) -> Option<PipelineJobsResult> {
		let state = self.state.lock().ok()?;
		match state.as_ref()? {
			PipelineJobsJobState::Response(r) => Some(r.clone()),
			PipelineJobsJobState::Request { .. } => None,
		}
	}

	fn fetch(
		remote: &GitLabRemote,
		pipeline_id: u64,
	) -> Result<Vec<Job>, Error> {
		let client = GitLabClient::from_env(remote.clone())?;
		runtime::block_on(client.pipeline_jobs(pipeline_id))
	}
}

impl AsyncJob for AsyncPipelineJobsJob {
	type Notification = AsyncGitLabNotification;
	type Progress = ();

	fn run(
		&mut self,
		_params: RunParams<Self::Notification, Self::Progress>,
	) -> GitResult<Self::Notification> {
		if let Ok(mut state) = self.state.lock() {
			*state = state.take().map(|state| match state {
				PipelineJobsJobState::Request {
					remote,
					pipeline_id,
				} => PipelineJobsJobState::Response(
					Self::fetch(&remote, pipeline_id)
						.map_err(|e| e.to_string()),
				),
				PipelineJobsJobState::Response(r) => {
					PipelineJobsJobState::Response(r)
				}
			});
		}
		Ok(AsyncGitLabNotification::PipelineJobs)
	}
}

enum TraceJobState {
	Request { remote: GitLabRemote, job_id: u64 },
	Response(TraceResult),
}

/// Fetches the raw trace (log) of a job, off the UI thread.
#[derive(Clone)]
pub struct AsyncTraceJob {
	state: Arc<Mutex<Option<TraceJobState>>>,
}

impl AsyncTraceJob {
	pub fn new(remote: GitLabRemote, job_id: u64) -> Self {
		Self {
			state: Arc::new(Mutex::new(Some(
				TraceJobState::Request { remote, job_id },
			))),
		}
	}

	pub fn result(&self) -> Option<TraceResult> {
		let state = self.state.lock().ok()?;
		match state.as_ref()? {
			TraceJobState::Response(r) => Some(r.clone()),
			TraceJobState::Request { .. } => None,
		}
	}

	fn fetch(
		remote: &GitLabRemote,
		job_id: u64,
	) -> Result<String, Error> {
		let client = GitLabClient::from_env(remote.clone())?;
		runtime::block_on(client.job_trace(job_id))
	}
}

impl AsyncJob for AsyncTraceJob {
	type Notification = AsyncGitLabNotification;
	type Progress = ();

	fn run(
		&mut self,
		_params: RunParams<Self::Notification, Self::Progress>,
	) -> GitResult<Self::Notification> {
		if let Ok(mut state) = self.state.lock() {
			*state = state.take().map(|state| match state {
				TraceJobState::Request { remote, job_id } => {
					TraceJobState::Response(
						Self::fetch(&remote, job_id)
							.map_err(|e| e.to_string()),
					)
				}
				TraceJobState::Response(r) => {
					TraceJobState::Response(r)
				}
			});
		}
		Ok(AsyncGitLabNotification::JobTrace)
	}
}

enum MrChangesJobState {
	Request { remote: GitLabRemote, iid: u64 },
	Response(MrChangesResult),
}

/// Fetches a merge request's changes (diff), off the UI thread.
#[derive(Clone)]
pub struct AsyncMrChangesJob {
	state: Arc<Mutex<Option<MrChangesJobState>>>,
}

impl AsyncMrChangesJob {
	pub fn new(remote: GitLabRemote, iid: u64) -> Self {
		Self {
			state: Arc::new(Mutex::new(Some(
				MrChangesJobState::Request { remote, iid },
			))),
		}
	}

	pub fn result(&self) -> Option<MrChangesResult> {
		let state = self.state.lock().ok()?;
		match state.as_ref()? {
			MrChangesJobState::Response(r) => Some(r.clone()),
			MrChangesJobState::Request { .. } => None,
		}
	}

	fn fetch(
		remote: &GitLabRemote,
		iid: u64,
	) -> Result<MrChanges, Error> {
		let client = GitLabClient::from_env(remote.clone())?;
		runtime::block_on(client.merge_request_changes(iid))
	}
}

impl AsyncJob for AsyncMrChangesJob {
	type Notification = AsyncGitLabNotification;
	type Progress = ();

	fn run(
		&mut self,
		_params: RunParams<Self::Notification, Self::Progress>,
	) -> GitResult<Self::Notification> {
		if let Ok(mut state) = self.state.lock() {
			*state = state.take().map(|state| match state {
				MrChangesJobState::Request { remote, iid } => {
					MrChangesJobState::Response(
						Self::fetch(&remote, iid)
							.map_err(|e| e.to_string()),
					)
				}
				MrChangesJobState::Response(r) => {
					MrChangesJobState::Response(r)
				}
			});
		}
		Ok(AsyncGitLabNotification::MrChanges)
	}
}

enum CommitsJobState {
	Request { remote: GitLabRemote },
	Response(CommitsResult),
}

/// Fetches recent commits (with pipeline status), off the UI thread.
#[derive(Clone)]
pub struct AsyncCommitsJob {
	state: Arc<Mutex<Option<CommitsJobState>>>,
}

impl AsyncCommitsJob {
	pub fn new(remote: GitLabRemote) -> Self {
		Self {
			state: Arc::new(Mutex::new(Some(
				CommitsJobState::Request { remote },
			))),
		}
	}

	pub fn result(&self) -> Option<CommitsResult> {
		let state = self.state.lock().ok()?;
		match state.as_ref()? {
			CommitsJobState::Response(r) => Some(r.clone()),
			CommitsJobState::Request { .. } => None,
		}
	}

	fn fetch(
		remote: &GitLabRemote,
	) -> Result<Vec<Commit>, Error> {
		let client = GitLabClient::from_env(remote.clone())?;
		runtime::block_on(client.commits(None))
	}
}

impl AsyncJob for AsyncCommitsJob {
	type Notification = AsyncGitLabNotification;
	type Progress = ();

	fn run(
		&mut self,
		_params: RunParams<Self::Notification, Self::Progress>,
	) -> GitResult<Self::Notification> {
		if let Ok(mut state) = self.state.lock() {
			*state = state.take().map(|state| match state {
				CommitsJobState::Request { remote } => {
					CommitsJobState::Response(
						Self::fetch(&remote)
							.map_err(|e| e.to_string()),
					)
				}
				CommitsJobState::Response(r) => {
					CommitsJobState::Response(r)
				}
			});
		}
		Ok(AsyncGitLabNotification::Commits)
	}
}

enum CommitStatusesJobState {
	Request { remote: GitLabRemote, sha: String },
	Response(CommitStatusesResult),
}

/// Fetches a commit's statuses, off the UI thread.
#[derive(Clone)]
pub struct AsyncCommitStatusesJob {
	state: Arc<Mutex<Option<CommitStatusesJobState>>>,
}

impl AsyncCommitStatusesJob {
	pub fn new(remote: GitLabRemote, sha: String) -> Self {
		Self {
			state: Arc::new(Mutex::new(Some(
				CommitStatusesJobState::Request { remote, sha },
			))),
		}
	}

	pub fn result(&self) -> Option<CommitStatusesResult> {
		let state = self.state.lock().ok()?;
		match state.as_ref()? {
			CommitStatusesJobState::Response(r) => Some(r.clone()),
			CommitStatusesJobState::Request { .. } => None,
		}
	}

	fn fetch(
		remote: &GitLabRemote,
		sha: &str,
	) -> Result<Vec<CommitStatus>, Error> {
		let client = GitLabClient::from_env(remote.clone())?;
		runtime::block_on(client.commit_statuses(sha))
	}
}

impl AsyncJob for AsyncCommitStatusesJob {
	type Notification = AsyncGitLabNotification;
	type Progress = ();

	fn run(
		&mut self,
		_params: RunParams<Self::Notification, Self::Progress>,
	) -> GitResult<Self::Notification> {
		if let Ok(mut state) = self.state.lock() {
			*state = state.take().map(|state| match state {
				CommitStatusesJobState::Request { remote, sha } => {
					CommitStatusesJobState::Response(
						Self::fetch(&remote, &sha)
							.map_err(|e| e.to_string()),
					)
				}
				CommitStatusesJobState::Response(r) => {
					CommitStatusesJobState::Response(r)
				}
			});
		}
		Ok(AsyncGitLabNotification::CommitStatuses)
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
	SetIssueLabels {
		iid: u64,
		labels: String,
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
	SetMergeRequestLabels {
		iid: u64,
		labels: String,
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
	RetryJob {
		id: u64,
	},
	CancelJob {
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
			GitLabAction::SetIssueLabels { iid, labels } => {
				client.set_issue_labels(iid, &labels).await?;
				Ok(format!("issue #{iid} labels updated"))
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
			GitLabAction::SetMergeRequestLabels { iid, labels } => {
				client
					.set_merge_request_labels(iid, &labels)
					.await?;
				Ok(format!("!{iid} labels updated"))
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
			GitLabAction::RetryJob { id } => {
				client.retry_job(id).await?;
				Ok(format!("job #{id} retried"))
			}
			GitLabAction::CancelJob { id } => {
				client.cancel_job(id).await?;
				Ok(format!("job #{id} canceled"))
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

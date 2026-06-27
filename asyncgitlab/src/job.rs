//! `AsyncJob` implementations so GitLab requests run on gitui's threadpool and
//! report completion over the same notification channel mechanism as git tasks.
//!
//! Pattern mirrors `asyncgit::AsyncFetchJob`: the job keeps its state behind an
//! `Arc<Mutex<..>>`, does the blocking work in `run`, stores the outcome, and
//! always returns `Ok(notification)`. The UI then calls `take_last()` on the
//! `AsyncSingleJob` and reads the result.

use crate::{
	client::{GitLabClient, MergeRequestScope},
	error::Error,
	remote::GitLabRemote,
	runtime,
	types::MergeRequest,
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
}

/// Result the UI reads after the job completes.
pub type MergeRequestsResult = Result<Vec<MergeRequest>, String>;

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

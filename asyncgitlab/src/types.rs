//! Lightweight deserialization targets for GitLab REST responses.
//!
//! Only the fields the UI needs are modeled; `serde` ignores the rest.

use serde::Deserialize;

/// Merge request state as reported by the API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MergeRequestState {
	Opened,
	Closed,
	Merged,
	Locked,
	#[serde(other)]
	Unknown,
}

/// A merge request, trimmed to what the list/detail views render.
#[derive(Debug, Clone, Deserialize)]
pub struct MergeRequest {
	pub iid: u64,
	pub title: String,
	pub state: MergeRequestState,
	pub source_branch: String,
	pub target_branch: String,
	#[serde(default)]
	pub draft: bool,
	#[serde(default)]
	pub web_url: String,
	#[serde(default)]
	pub description: Option<String>,
	#[serde(default)]
	pub author: Option<User>,
	#[serde(default)]
	pub upvotes: u32,
	#[serde(default)]
	pub downvotes: u32,
	#[serde(default)]
	pub user_notes_count: u32,
	#[serde(default)]
	pub detailed_merge_status: Option<String>,
	#[serde(default)]
	pub has_conflicts: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct User {
	pub username: String,
}

/// Issue state as reported by the API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IssueState {
	Opened,
	Closed,
	#[serde(other)]
	Unknown,
}

/// An issue, trimmed to what the list/detail views render.
#[derive(Debug, Clone, Deserialize)]
pub struct Issue {
	pub iid: u64,
	pub title: String,
	pub state: IssueState,
	#[serde(default)]
	pub description: Option<String>,
	#[serde(default)]
	pub web_url: String,
	#[serde(default)]
	pub author: Option<User>,
	#[serde(default)]
	pub labels: Vec<String>,
	#[serde(default)]
	pub upvotes: u32,
	#[serde(default)]
	pub user_notes_count: u32,
	#[serde(default)]
	pub assignees: Vec<User>,
}

/// A project label.
#[derive(Debug, Clone, Deserialize)]
pub struct Label {
	pub name: String,
	#[serde(default)]
	pub color: String,
}

/// A single column (list) of an issue board.
#[derive(Debug, Clone, Deserialize)]
pub struct BoardList {
	pub id: u64,
	#[serde(default)]
	pub label: Option<Label>,
	#[serde(default)]
	pub position: i64,
}

/// An issue board: an ordered set of label-backed lists.
#[derive(Debug, Clone, Deserialize)]
pub struct Board {
	pub id: u64,
	#[serde(default)]
	pub name: String,
	#[serde(default)]
	pub lists: Vec<BoardList>,
}

/// A note (comment) on an issue or merge request.
#[derive(Debug, Clone, Deserialize)]
pub struct Note {
	pub id: u64,
	pub body: String,
	#[serde(default)]
	pub author: Option<User>,
	#[serde(default)]
	pub system: bool,
	#[serde(default)]
	pub created_at: String,
}

/// CI status, shared by pipelines and individual jobs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CiStatus {
	Created,
	#[serde(rename = "waiting_for_resource")]
	WaitingForResource,
	Preparing,
	Pending,
	Running,
	Success,
	Failed,
	Canceled,
	Skipped,
	Manual,
	Scheduled,
	#[serde(other)]
	Unknown,
}

/// Backwards-compatible alias: pipelines historically used `PipelineStatus`.
pub type PipelineStatus = CiStatus;

#[derive(Debug, Clone, Deserialize)]
pub struct Pipeline {
	pub id: u64,
	pub status: CiStatus,
	#[serde(default)]
	pub web_url: String,
	#[serde(default)]
	pub r#ref: Option<String>,
	#[serde(default)]
	pub sha: Option<String>,
}

/// A single CI job within a pipeline.
#[derive(Debug, Clone, Deserialize)]
pub struct Job {
	pub id: u64,
	pub name: String,
	pub status: CiStatus,
	#[serde(default)]
	pub stage: String,
	#[serde(default)]
	pub web_url: String,
	#[serde(default)]
	pub allow_failure: bool,
}

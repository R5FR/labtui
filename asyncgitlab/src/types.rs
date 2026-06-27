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
	pub author: Option<User>,
	#[serde(default)]
	pub upvotes: u32,
	#[serde(default)]
	pub detailed_merge_status: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct User {
	pub username: String,
}

/// Pipeline status for a branch/commit, for the CI badge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PipelineStatus {
	Created,
	Pending,
	Running,
	Success,
	Failed,
	Canceled,
	Skipped,
	Manual,
	#[serde(other)]
	Unknown,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Pipeline {
	pub id: u64,
	pub status: PipelineStatus,
	#[serde(default)]
	pub web_url: String,
}

//! Derivation of issue-board columns from board lists + issues.
//!
//! Mirrors how the GitLab board groups issues: a leading "Open" backlog,
//! one column per board list (in `position` order, keyed by its label),
//! and a trailing "Closed" column. An opened issue lands in every list whose
//! label it carries; opened issues matching no list label fall into "Open".

use crate::types::{BoardList, Issue, IssueState};

/// One rendered board column with its issues.
#[derive(Debug, Clone)]
pub struct BoardColumn {
	pub title: String,
	pub issues: Vec<Issue>,
}

/// Bucket `issues` into board columns following `lists`.
pub fn build_board(
	lists: &[BoardList],
	issues: Vec<Issue>,
) -> Vec<BoardColumn> {
	// list columns, in position order, that actually carry a label
	let mut label_lists: Vec<&BoardList> =
		lists.iter().filter(|l| l.label.is_some()).collect();
	label_lists.sort_by_key(|l| l.position);

	let label_names: Vec<&str> = label_lists
		.iter()
		.filter_map(|l| l.label.as_ref().map(|lb| lb.name.as_str()))
		.collect();

	let mut open: Vec<Issue> = Vec::new();
	let mut per_list: Vec<Vec<Issue>> =
		vec![Vec::new(); label_lists.len()];
	let mut closed: Vec<Issue> = Vec::new();

	for issue in issues {
		if issue.state == IssueState::Closed {
			closed.push(issue);
			continue;
		}

		let matches: Vec<usize> = label_names
			.iter()
			.enumerate()
			.filter(|(_, name)| {
				issue.labels.iter().any(|l| l == *name)
			})
			.map(|(i, _)| i)
			.collect();

		if matches.is_empty() {
			open.push(issue);
		} else {
			for &i in &matches {
				per_list[i].push(issue.clone());
			}
		}
	}

	let mut columns = Vec::with_capacity(label_lists.len() + 2);
	columns.push(BoardColumn {
		title: "Open".to_string(),
		issues: open,
	});
	for (i, list) in label_lists.iter().enumerate() {
		let title = list
			.label
			.as_ref()
			.map_or("?", |l| l.name.as_str())
			.to_string();
		columns.push(BoardColumn {
			title,
			issues: std::mem::take(&mut per_list[i]),
		});
	}
	columns.push(BoardColumn {
		title: "Closed".to_string(),
		issues: closed,
	});
	columns
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::types::{IssueState, Label};

	fn issue(iid: u64, state: IssueState, labels: &[&str]) -> Issue {
		Issue {
			iid,
			title: format!("issue {iid}"),
			state,
			description: None,
			web_url: String::new(),
			author: None,
			labels: labels.iter().map(|s| (*s).to_string()).collect(),
			upvotes: 0,
			user_notes_count: 0,
			assignees: Vec::new(),
		}
	}

	fn list(id: u64, label: &str, position: i64) -> BoardList {
		BoardList {
			id,
			label: Some(Label {
				name: label.to_string(),
				color: String::new(),
			}),
			position,
		}
	}

	#[test]
	fn buckets_open_lists_and_closed() {
		let lists = vec![list(1, "Doing", 1), list(2, "Review", 0)];
		let issues = vec![
			issue(1, IssueState::Opened, &[]), // backlog
			issue(2, IssueState::Opened, &["Doing"]),
			issue(3, IssueState::Opened, &["Review"]),
			issue(4, IssueState::Closed, &["Doing"]),
		];

		let cols = build_board(&lists, issues);

		// Open, Review (pos 0), Doing (pos 1), Closed
		assert_eq!(cols.len(), 4);
		assert_eq!(cols[0].title, "Open");
		assert_eq!(cols[0].issues.len(), 1);
		assert_eq!(cols[1].title, "Review");
		assert_eq!(cols[1].issues[0].iid, 3);
		assert_eq!(cols[2].title, "Doing");
		assert_eq!(cols[2].issues[0].iid, 2);
		assert_eq!(cols[3].title, "Closed");
		assert_eq!(cols[3].issues[0].iid, 4);
	}

	#[test]
	fn issue_with_multiple_labels_appears_in_each() {
		let lists = vec![list(1, "A", 0), list(2, "B", 1)];
		let issues = vec![issue(1, IssueState::Opened, &["A", "B"])];

		let cols = build_board(&lists, issues);

		assert_eq!(cols[0].issues.len(), 0); // Open empty
		assert_eq!(cols[1].issues.len(), 1); // A
		assert_eq!(cols[2].issues.len(), 1); // B
	}

	#[test]
	fn no_lists_yields_open_and_closed() {
		let issues = vec![
			issue(1, IssueState::Opened, &[]),
			issue(2, IssueState::Closed, &[]),
		];
		let cols = build_board(&[], issues);
		assert_eq!(cols.len(), 2);
		assert_eq!(cols[0].title, "Open");
		assert_eq!(cols[1].title, "Closed");
	}
}

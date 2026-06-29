use crate::{
	app::Environment,
	components::{
		visibility_blocking, CommandBlocking, CommandInfo, Component,
		DrawableComponent, EventState, InputType, TextInputComponent,
	},
	keys::{key_match, SharedKeyConfig},
	strings,
	ui::style::SharedTheme,
};
use anyhow::Result;
use asyncgit::{
	asyncjob::AsyncSingleJob,
	sync::{get_default_remote, get_remote_url, RepoPathRef},
};
use asyncgitlab::{
	has_token, store_token, AsyncActionJob, AsyncGitLabNotification,
	AsyncMergeRequestsJob, AsyncMrChangesJob, AsyncMrDetailJob,
	ChangedFile, CiStatus, GitLabAction, GitLabRemote, MergeRequest,
	MergeRequestScope, MergeRequestState, Note, StateEvent,
};
use crossterm::event::{Event, KeyCode, KeyEvent};
use ratatui::{
	layout::{Alignment, Constraint, Direction, Layout, Rect},
	text::{Line, Span},
	widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
	Frame,
};

/// Loading state of a fetched payload.
enum Load<T> {
	Loading,
	Loaded(T),
	Error(String),
}

/// Loaded detail of a single merge request: the MR plus its comment thread.
struct DetailData {
	mr: MergeRequest,
	notes: Vec<Note>,
}

pub struct MergeRequestsTab {
	visible: bool,
	remote: Option<GitLabRemote>,
	list: Load<Vec<MergeRequest>>,
	selection: usize,
	/// open MR detail view, if any
	detail: Option<Load<DetailData>>,
	detail_iid: Option<u64>,
	detail_scroll: u16,
	/// diff (changes) view over the detail, if open
	changes: Option<Load<Vec<ChangedFile>>>,
	changes_scroll: u16,
	/// case-insensitive substring filter applied to the list
	filter: String,
	/// transient one-line feedback after a write action
	status_msg: Option<String>,
	/// error from storing a token in the keyring
	token_error: Option<String>,
	async_mrs: AsyncSingleJob<AsyncMergeRequestsJob>,
	async_detail: AsyncSingleJob<AsyncMrDetailJob>,
	async_changes: AsyncSingleJob<AsyncMrChangesJob>,
	async_action: AsyncSingleJob<AsyncActionJob>,
	token_input: TextInputComponent,
	comment_input: TextInputComponent,
	label_input: TextInputComponent,
	filter_input: TextInputComponent,
	theme: SharedTheme,
	key_config: SharedKeyConfig,
}

impl MergeRequestsTab {
	pub fn new(env: &Environment) -> Self {
		let remote = detect_gitlab_remote(&env.repo);

		let token_input = TextInputComponent::new(
			env,
			"GitLab token",
			"paste a token with api scope, then press [Enter]",
			false,
		)
		.with_input_type(InputType::Password);

		let comment_input = TextInputComponent::new(
			env,
			"New comment",
			"comment body, then press [Enter]",
			true,
		);
		let label_input = TextInputComponent::new(
			env,
			"Labels",
			"comma-separated labels, then press [Enter]",
			false,
		);
		let filter_input = TextInputComponent::new(
			env,
			"Filter",
			"type to filter, then press [Enter]",
			false,
		);

		Self {
			visible: false,
			remote,
			list: Load::Loading,
			selection: 0,
			detail: None,
			detail_iid: None,
			detail_scroll: 0,
			changes: None,
			changes_scroll: 0,
			filter: String::new(),
			status_msg: None,
			token_error: None,
			async_mrs: AsyncSingleJob::new(env.sender_gitlab.clone()),
			async_detail: AsyncSingleJob::new(
				env.sender_gitlab.clone(),
			),
			async_changes: AsyncSingleJob::new(
				env.sender_gitlab.clone(),
			),
			async_action: AsyncSingleJob::new(
				env.sender_gitlab.clone(),
			),
			token_input,
			comment_input,
			label_input,
			filter_input,
			theme: env.theme.clone(),
			key_config: env.key_config.clone(),
		}
	}

	pub fn update(&mut self) {
		if self.is_visible() {
			self.ensure_load();
		}
	}

	fn token_available(&self) -> bool {
		self.remote
			.as_ref()
			.is_some_and(|r| has_token(&r.host))
	}

	fn ensure_load(&mut self) {
		if !self.token_available() {
			return;
		}
		if self.token_input.is_visible() {
			self.token_input.hide();
		}
		let Some(remote) = self.remote.clone() else {
			return;
		};
		if self.async_action.is_pending() {
			return;
		}
		if matches!(self.list, Load::Loading)
			&& !self.async_mrs.is_pending()
		{
			self.async_mrs.spawn(AsyncMergeRequestsJob::new(
				remote,
				MergeRequestScope::Opened,
			));
		}
	}

	fn reload(&mut self) {
		self.list = Load::Loading;
		self.ensure_load();
	}

	fn show_token_prompt(&mut self) {
		if !self.token_input.is_visible() {
			self.token_input.clear();
			let _ = self.token_input.show();
		}
	}

	/// True while the user is entering text (used to block global quit).
	pub fn is_editing(&self) -> bool {
		self.token_input.is_visible()
			|| self.comment_input.is_visible()
			|| self.label_input.is_visible()
			|| self.filter_input.is_visible()
	}

	fn submit_token(&mut self) {
		let token = self.token_input.get_text().trim().to_string();
		if token.is_empty() {
			return;
		}
		let Some(host) =
			self.remote.as_ref().map(|r| r.host.clone())
		else {
			return;
		};
		match store_token(&host, &token) {
			Ok(()) => {
				self.token_input.clear();
				self.token_input.hide();
				self.token_error = None;
				self.list = Load::Loading;
				self.ensure_load();
			}
			Err(e) => {
				self.token_input.hide();
				self.token_error = Some(format!(
					"could not store token in keyring: {e}"
				));
			}
		}
	}

	fn open_detail(&mut self) {
		let Some(iid) = self.selected_mr().map(|m| m.iid) else {
			return;
		};
		let Some(remote) = self.remote.clone() else {
			return;
		};
		self.detail_iid = Some(iid);
		self.detail_scroll = 0;
		self.detail = Some(Load::Loading);
		self.async_detail
			.spawn(AsyncMrDetailJob::new(remote, iid));
	}

	fn close_detail(&mut self) {
		self.detail = None;
		self.detail_iid = None;
		self.detail_scroll = 0;
	}

	fn reload_detail(&mut self) {
		let (Some(iid), Some(remote)) =
			(self.detail_iid, self.remote.clone())
		else {
			return;
		};
		self.detail = Some(Load::Loading);
		self.async_detail
			.spawn(AsyncMrDetailJob::new(remote, iid));
	}

	const fn detail_open(&self) -> bool {
		self.detail.is_some()
	}

	const fn scroll_detail(&mut self, down: bool) {
		if down {
			self.detail_scroll = self.detail_scroll.saturating_add(1);
		} else {
			self.detail_scroll = self.detail_scroll.saturating_sub(1);
		}
	}

	fn show_comment_prompt(&mut self) {
		if !self.comment_input.is_visible() {
			self.comment_input.clear();
			let _ = self.comment_input.show();
		}
	}

	fn submit_comment(&mut self) {
		let body = self.comment_input.get_text().trim().to_string();
		self.comment_input.clear();
		self.comment_input.hide();
		let Some(iid) = self.detail_iid else {
			return;
		};
		if body.is_empty() {
			return;
		}
		self.spawn_action(GitLabAction::CreateMergeRequestNote {
			iid,
			body,
		});
	}

	/// The iid the action keys (merge/approve/…) apply to: the detail view's
	/// MR, else the selected list row.
	fn action_iid(&self) -> Option<u64> {
		if let Some(iid) = self.detail_iid {
			return Some(iid);
		}
		self.selected_mr().map(|m| m.iid)
	}

	fn current_mr(&self) -> Option<&MergeRequest> {
		if let Some(Load::Loaded(d)) = &self.detail {
			return Some(&d.mr);
		}
		self.selected_mr()
	}

	fn spawn_action(&mut self, action: GitLabAction) {
		let Some(remote) = self.remote.clone() else {
			return;
		};
		if self.async_action.is_pending() {
			return;
		}
		self.status_msg = Some("working…".to_string());
		self.async_action
			.spawn(AsyncActionJob::new(remote, action));
	}

	/// Close the current MR, or reopen it if closed (no-op when merged).
	fn toggle_state(&mut self) {
		let event = match self.current_mr().map(|m| m.state) {
			Some(MergeRequestState::Closed) => StateEvent::Reopen,
			Some(
				MergeRequestState::Opened
				| MergeRequestState::Locked,
			) => StateEvent::Close,
			_ => return,
		};
		if let Some(iid) = self.action_iid() {
			self.spawn_action(GitLabAction::SetMergeRequestState {
				iid,
				event,
			});
		}
	}

	fn open_in_browser(&mut self) {
		let url = self.current_mr().map(|m| m.web_url.clone());
		let Some(url) = url.filter(|u| !u.is_empty()) else {
			return;
		};
		if let Err(e) = crate::open_browser::open_in_browser(&url) {
			self.status_msg = Some(format!("error: {e}"));
		}
	}

	const fn changes_open(&self) -> bool {
		self.changes.is_some()
	}

	/// Open the diff (changes) view for the MR in the detail view.
	fn open_changes(&mut self) {
		let (Some(iid), Some(remote)) =
			(self.detail_iid, self.remote.clone())
		else {
			return;
		};
		self.changes_scroll = 0;
		self.changes = Some(Load::Loading);
		self.async_changes
			.spawn(AsyncMrChangesJob::new(remote, iid));
	}

	fn close_changes(&mut self) {
		self.changes = None;
		self.changes_scroll = 0;
	}

	const fn scroll_changes(&mut self, down: bool) {
		if down {
			self.changes_scroll =
				self.changes_scroll.saturating_add(1);
		} else {
			self.changes_scroll =
				self.changes_scroll.saturating_sub(1);
		}
	}

	/// Open the label editor, pre-filled with the current MR's labels.
	fn show_label_prompt(&mut self) {
		if self.label_input.is_visible() {
			return;
		}
		let labels = self
			.current_mr()
			.map(|m| m.labels.join(", "))
			.unwrap_or_default();
		self.label_input.set_text(labels);
		let _ = self.label_input.show();
	}

	fn submit_labels(&mut self) {
		let labels = self.label_input.get_text().trim().to_string();
		self.label_input.hide();
		let Some(iid) = self.action_iid() else {
			return;
		};
		self.spawn_action(GitLabAction::SetMergeRequestLabels {
			iid,
			labels,
		});
	}

	fn show_filter_prompt(&mut self) {
		if self.filter_input.is_visible() {
			return;
		}
		self.filter_input.set_text(self.filter.clone());
		let _ = self.filter_input.show();
	}

	fn submit_filter(&mut self) {
		self.filter = self.filter_input.get_text().trim().to_string();
		self.filter_input.hide();
		self.selection = 0;
	}

	/// handle a finished GitLab job
	pub fn update_gitlab(&mut self, ev: AsyncGitLabNotification) {
		match ev {
			AsyncGitLabNotification::MergeRequests => {
				if let Some(job) = self.async_mrs.take_last() {
					if let Some(result) = job.result() {
						self.list = match result {
							Ok(mrs) => Load::Loaded(mrs),
							Err(e) => Load::Error(e),
						};
						self.clamp_selection();
					}
				}
			}
			AsyncGitLabNotification::MrDetail => {
				if let Some(job) = self.async_detail.take_last() {
					if let Some(result) = job.result() {
						self.detail = Some(match result {
							Ok((mr, notes)) => {
								Load::Loaded(DetailData { mr, notes })
							}
							Err(e) => Load::Error(e),
						});
					}
				}
			}
			AsyncGitLabNotification::Action => {
				if let Some(job) = self.async_action.take_last() {
					if let Some(result) = job.result() {
						self.status_msg = Some(match result {
							Ok(msg) => msg,
							Err(e) => format!("error: {e}"),
						});
						self.reload();
						if self.detail_open() {
							self.reload_detail();
						}
					}
				}
			}
			AsyncGitLabNotification::MrChanges => {
				if let Some(job) = self.async_changes.take_last() {
					// ignore the result if the user already closed the diff
					if self.changes.is_some() {
						if let Some(result) = job.result() {
							self.changes = Some(match result {
								Ok(c) => Load::Loaded(c.changes),
								Err(e) => Load::Error(e),
							});
						}
					}
				}
			}
			AsyncGitLabNotification::Issues
			| AsyncGitLabNotification::Board
			| AsyncGitLabNotification::IssueDetail
			| AsyncGitLabNotification::Pipelines
			| AsyncGitLabNotification::PipelineJobs
			| AsyncGitLabNotification::JobTrace
			| AsyncGitLabNotification::Commits
			| AsyncGitLabNotification::CommitStatuses => {}
		}
	}

	pub fn any_work_pending(&self) -> bool {
		self.async_mrs.is_pending()
			|| self.async_detail.is_pending()
			|| self.async_changes.is_pending()
			|| self.async_action.is_pending()
	}

	fn loaded(&self) -> Option<&[MergeRequest]> {
		match &self.list {
			Load::Loaded(mrs) => Some(mrs),
			_ => None,
		}
	}

	fn matches_filter(&self, mr: &MergeRequest) -> bool {
		if self.filter.is_empty() {
			return true;
		}
		let f = self.filter.to_lowercase();
		mr.title.to_lowercase().contains(&f)
			|| mr.source_branch.to_lowercase().contains(&f)
			|| mr.target_branch.to_lowercase().contains(&f)
			|| mr
				.author
				.as_ref()
				.is_some_and(|a| {
					a.username.to_lowercase().contains(&f)
				})
			|| mr.labels.iter().any(|l| l.to_lowercase().contains(&f))
			|| format!("!{}", mr.iid).contains(&f)
	}

	fn filtered(&self) -> Vec<&MergeRequest> {
		self.loaded().map_or_else(Vec::new, |mrs| {
			mrs.iter().filter(|m| self.matches_filter(m)).collect()
		})
	}

	fn selected_mr(&self) -> Option<&MergeRequest> {
		self.filtered().get(self.selection).copied()
	}

	fn clamp_selection(&mut self) {
		let len = self.filtered().len();
		if len == 0 {
			self.selection = 0;
		} else if self.selection >= len {
			self.selection = len - 1;
		}
	}

	fn move_selection(&mut self, down: bool) {
		let len = self.filtered().len();
		if len == 0 {
			return;
		}
		if down {
			self.selection = (self.selection + 1) % len;
		} else {
			self.selection =
				self.selection.checked_sub(1).unwrap_or(len - 1);
		}
	}

	fn draw_message(&self, f: &mut Frame, rect: Rect, msg: &str) {
		let block = Block::default()
			.borders(Borders::ALL)
			.title(self.title());
		let p = Paragraph::new(msg)
			.block(block)
			.alignment(Alignment::Center)
			.style(self.theme.text(true, false));
		f.render_widget(p, rect);
	}

	fn title(&self) -> String {
		self.remote.as_ref().map_or_else(
			|| "Merge Requests".to_string(),
			|r| format!("Merge Requests · {}", r.project_path),
		)
	}

	fn host(&self) -> &str {
		self.remote.as_ref().map_or("", |r| r.host.as_str())
	}

	fn split_footer(&self, rect: Rect) -> (Rect, Option<Rect>) {
		if self.status_msg.is_some() {
			let chunks = Layout::default()
				.direction(Direction::Vertical)
				.constraints([
					Constraint::Min(1),
					Constraint::Length(1),
				])
				.split(rect);
			(chunks[0], Some(chunks[1]))
		} else {
			(rect, None)
		}
	}

	fn draw_footer(&self, f: &mut Frame, footer: Option<Rect>) {
		if let (Some(rect), Some(msg)) =
			(footer, self.status_msg.as_deref())
		{
			f.render_widget(
				Paragraph::new(msg)
					.style(self.theme.text(true, false)),
				rect,
			);
		}
	}

	const fn marker(mr: &MergeRequest) -> &'static str {
		match mr.state {
			MergeRequestState::Merged => "✓",
			MergeRequestState::Closed => "✗",
			_ if mr.draft => "◐",
			_ => "●",
		}
	}

	fn render_list(&self, f: &mut Frame, rect: Rect) {
		let (list_area, footer) = self.split_footer(rect);
		let mrs = self.filtered();

		let items: Vec<ListItem> = mrs
			.iter()
			.enumerate()
			.map(|(i, mr)| {
				let selected = i == self.selection;
				let author = mr
					.author
					.as_ref()
					.map_or_else(String::new, |a| {
						format!(" @{}", a.username)
					});
				let line = Line::from(vec![Span::styled(
					format!(
						"{} !{}  {}  ({} → {}){author}",
						Self::marker(mr),
						mr.iid,
						mr.title,
						mr.source_branch,
						mr.target_branch,
					),
					self.theme.text(true, selected),
				)]);
				ListItem::new(line)
			})
			.collect();

		let title = if self.filter.is_empty() {
			self.title()
		} else {
			format!("{}  (filter: {})", self.title(), self.filter)
		};
		let list = List::new(items).block(
			Block::default().borders(Borders::ALL).title(title),
		);
		f.render_widget(list, list_area);
		self.draw_footer(f, footer);
	}

	fn render_detail(
		&self,
		f: &mut Frame,
		rect: Rect,
		data: &DetailData,
	) {
		let (area, footer) = self.split_footer(rect);
		let lines = self.detail_lines(data);
		let title = format!(
			"MR !{}  ·  [esc] back  [d] diff  [l] labels  [n] comment  [m] merge  [a]/[u] approve  [b] rebase  [c] close  [o] open",
			data.mr.iid
		);
		let block = Block::default()
			.borders(Borders::ALL)
			.title(title);
		f.render_widget(
			Paragraph::new(lines)
				.block(block)
				.wrap(Wrap { trim: false })
				.scroll((self.detail_scroll, 0)),
			area,
		);
		self.draw_footer(f, footer);
	}

	fn detail_lines(&self, data: &DetailData) -> Vec<Line<'static>> {
		let style = self.theme.text(true, false);
		let header = self.theme.text(true, true);
		let mr = &data.mr;

		let state = match mr.state {
			MergeRequestState::Opened => "open",
			MergeRequestState::Closed => "closed",
			MergeRequestState::Merged => "merged",
			MergeRequestState::Locked => "locked",
			MergeRequestState::Unknown => "?",
		};
		let author = mr
			.author
			.as_ref()
			.map_or_else(String::new, |a| {
				format!("  by @{}", a.username)
			});

		let mut lines: Vec<Line> = Vec::new();
		lines.push(Line::styled(
			format!(
				"{} !{}  {}",
				Self::marker(mr),
				mr.iid,
				mr.title
			),
			header,
		));
		lines.push(Line::styled(
			format!(
				"[{state}]{}{author}   👍{}",
				if mr.draft { "  draft" } else { "" },
				mr.upvotes
			),
			style,
		));
		lines.push(Line::styled(
			format!("{} → {}", mr.source_branch, mr.target_branch),
			style,
		));
		if let Some(p) = &mr.head_pipeline {
			lines.push(Line::styled(
				format!(
					"pipeline: {} #{}",
					ci_marker(p.status),
					p.id
				),
				style,
			));
		}
		if !mr.labels.is_empty() {
			lines.push(Line::styled(
				format!("labels: {}", mr.labels.join(", ")),
				style,
			));
		}
		if let Some(status) = &mr.detailed_merge_status {
			lines.push(Line::styled(
				format!("merge status: {status}"),
				style,
			));
		}
		if mr.has_conflicts {
			lines.push(Line::styled("⚠ has conflicts", style));
		}
		lines.push(Line::raw(""));
		match mr
			.description
			.as_deref()
			.filter(|d| !d.trim().is_empty())
		{
			Some(desc) => {
				for l in desc.lines() {
					lines.push(Line::styled(l.to_string(), style));
				}
			}
			None => {
				lines.push(Line::styled("(no description)", style));
			}
		}

		lines.extend(self.comment_lines(&data.notes));
		lines
	}

	/// Render the (non-system) notes of an issue/MR as text lines.
	fn comment_lines(&self, notes: &[Note]) -> Vec<Line<'static>> {
		let style = self.theme.text(true, false);
		let header = self.theme.text(true, true);
		let comments: Vec<&Note> =
			notes.iter().filter(|n| !n.system).collect();

		let mut lines: Vec<Line> = Vec::new();
		lines.push(Line::raw(""));
		lines.push(Line::styled(
			format!("── Comments ({}) ──", comments.len()),
			header,
		));
		if comments.is_empty() {
			lines.push(Line::styled("(no comments)", style));
		}
		for note in comments {
			lines.push(Line::raw(""));
			let who = note.author.as_ref().map_or_else(
				|| "?".to_string(),
				|a| format!("@{}", a.username),
			);
			let when =
				note.created_at.split('T').next().unwrap_or("");
			lines.push(Line::styled(
				format!("{who} · {when}"),
				header,
			));
			for l in note.body.lines() {
				lines.push(Line::styled(l.to_string(), style));
			}
		}
		lines
	}

	fn render_changes(
		&self,
		f: &mut Frame,
		rect: Rect,
		files: &[ChangedFile],
	) {
		let (area, footer) = self.split_footer(rect);
		let style = self.theme.text(true, false);
		let header = self.theme.text(true, true);

		let mut lines: Vec<Line> = Vec::new();
		lines.push(Line::styled(
			format!("{} changed file(s)", files.len()),
			header,
		));
		for file in files {
			lines.push(Line::raw(""));
			let tag = if file.new_file {
				"added"
			} else if file.deleted_file {
				"deleted"
			} else if file.renamed_file {
				"renamed"
			} else {
				"modified"
			};
			lines.push(Line::styled(
				format!("▸ {} ({tag})", file.new_path),
				header,
			));
			for l in file.diff.lines() {
				lines.push(Line::styled(l, style));
			}
		}

		let block = Block::default()
			.borders(Borders::ALL)
			.title("Changes  ·  [↑/↓] scroll  [esc] back");
		f.render_widget(
			Paragraph::new(lines)
				.block(block)
				.wrap(Wrap { trim: false })
				.scroll((self.changes_scroll, 0)),
			area,
		);
		self.draw_footer(f, footer);
	}
}

impl DrawableComponent for MergeRequestsTab {
	fn draw(&self, f: &mut Frame, rect: Rect) -> Result<()> {
		if self.remote.is_none() {
			self.draw_message(
				f,
				rect,
				"No GitLab remote detected for this repository.",
			);
			return Ok(());
		}

		if !self.token_available() {
			if self.token_input.is_visible() {
				self.draw_message(
					f,
					rect,
					&format!(
						"A GitLab token is required for {}.",
						self.host()
					),
				);
				self.token_input.draw(f, rect)?;
			} else if let Some(err) = &self.token_error {
				self.draw_message(
					f,
					rect,
					&format!(
						"{err}\n\nPress [Enter] to try again."
					),
				);
			} else {
				self.draw_message(
					f,
					rect,
					&strings::gitlab_token_help(self.host(), true),
				);
			}
			return Ok(());
		}

		// diff view takes over while open
		if let Some(changes) = &self.changes {
			match changes {
				Load::Loading => {
					self.draw_message(f, rect, "Loading changes…");
				}
				Load::Error(e) => self.draw_message(
					f,
					rect,
					&format!("Failed to load changes:\n{e}"),
				),
				Load::Loaded(files) => {
					self.render_changes(f, rect, files);
				}
			}
			return Ok(());
		}

		if let Some(detail) = &self.detail {
			match detail {
				Load::Loading => {
					self.draw_message(f, rect, "Loading merge request…");
				}
				Load::Error(e) => self.draw_message(
					f,
					rect,
					&format!(
						"Failed to load merge request:\n{e}\n\nPress [Esc] to go back."
					),
				),
				Load::Loaded(data) => {
					self.render_detail(f, rect, data);
				}
			}
			if self.comment_input.is_visible() {
				self.comment_input.draw(f, rect)?;
			}
			if self.label_input.is_visible() {
				self.label_input.draw(f, rect)?;
			}
			return Ok(());
		}

		match &self.list {
			Load::Loading => {
				self.draw_message(f, rect, "Loading merge requests…");
			}
			Load::Error(e) => self.draw_message(
				f,
				rect,
				&format!(
					"Failed to load merge requests:\n{e}\n\nPress [r] to retry."
				),
			),
			Load::Loaded(mrs) if mrs.is_empty() => {
				self.draw_message(f, rect, "No open merge requests.");
			}
			Load::Loaded(_) if self.filtered().is_empty() => {
				self.draw_message(
					f,
					rect,
					"No matching merge requests.\n\nPress [f] to change the filter.",
				);
			}
			Load::Loaded(_) => self.render_list(f, rect),
		}

		if self.filter_input.is_visible() {
			self.filter_input.draw(f, rect)?;
		}

		Ok(())
	}
}

impl Component for MergeRequestsTab {
	fn commands(
		&self,
		out: &mut Vec<CommandInfo>,
		force_all: bool,
	) -> CommandBlocking {
		if self.visible || force_all {
			out.push(CommandInfo::new(
				strings::commands::scroll(&self.key_config),
				self.loaded().is_some_and(|m| !m.is_empty())
					|| self.detail_open(),
				true,
			));
			out.push(CommandInfo::new(
				strings::commands::mr_open(&self.key_config),
				self.selected_mr().is_some(),
				self.loaded().is_some(),
			));
			out.push(CommandInfo::new(
				strings::commands::mr_merge(&self.key_config),
				self.current_mr().is_some(),
				self.detail_open() || self.loaded().is_some(),
			));
			out.push(CommandInfo::new(
				strings::commands::gitlab_browser(&self.key_config),
				self.current_mr().is_some(),
				self.detail_open() || self.loaded().is_some(),
			));
		}

		visibility_blocking(self)
	}

	fn event(&mut self, ev: &Event) -> Result<EventState> {
		if !self.is_visible() {
			return Ok(EventState::NotConsumed);
		}

		if let Some(state) = self.input_event(ev)? {
			return Ok(state);
		}

		if self.changes_open() {
			if let Event::Key(k) = ev {
				if key_match(k, self.key_config.keys.exit_popup) {
					self.close_changes();
				} else if key_match(k, self.key_config.keys.move_down)
				{
					self.scroll_changes(true);
				} else if key_match(k, self.key_config.keys.move_up) {
					self.scroll_changes(false);
				}
			}
			return Ok(EventState::Consumed);
		}

		if self.detail_open() {
			if let Event::Key(k) = ev {
				self.detail_event(k);
			}
			return Ok(EventState::Consumed);
		}

		if let Event::Key(k) = ev {
			let token_missing = !self.token_available();

			if key_match(k, self.key_config.keys.move_down) {
				self.move_selection(true);
				return Ok(EventState::Consumed);
			} else if key_match(k, self.key_config.keys.move_up) {
				self.move_selection(false);
				return Ok(EventState::Consumed);
			} else if key_match(k, self.key_config.keys.enter) {
				if token_missing {
					self.show_token_prompt();
				} else if self.selected_mr().is_some() {
					self.open_detail();
				}
				return Ok(EventState::Consumed);
			} else if matches!(k.code, KeyCode::Char('o'))
				&& self.selected_mr().is_some()
			{
				self.open_in_browser();
				return Ok(EventState::Consumed);
			} else if matches!(k.code, KeyCode::Char('f'))
				&& !token_missing
			{
				self.show_filter_prompt();
				return Ok(EventState::Consumed);
			} else if matches!(k.code, KeyCode::Char('r'))
				&& !token_missing
			{
				self.reload();
				return Ok(EventState::Consumed);
			}
		}

		Ok(EventState::NotConsumed)
	}

	fn is_visible(&self) -> bool {
		self.visible
	}

	fn hide(&mut self) {
		self.visible = false;
	}

	fn show(&mut self) -> Result<()> {
		self.visible = true;
		self.ensure_load();
		Ok(())
	}
}

impl MergeRequestsTab {
	/// Route a key event to the active text input. Returns `Some(Consumed)`
	/// when an input handled it, `None` when none is open.
	fn input_event(
		&mut self,
		ev: &Event,
	) -> Result<Option<EventState>> {
		if self.token_input.is_visible() {
			if !self.token_input.event(ev)?.is_consumed() {
				if let Event::Key(k) = ev {
					if key_match(k, self.key_config.keys.enter) {
						self.submit_token();
					} else if key_match(
						k,
						self.key_config.keys.exit_popup,
					) {
						self.token_input.hide();
					}
				}
			}
			return Ok(Some(EventState::Consumed));
		}

		if self.comment_input.is_visible() {
			if !self.comment_input.event(ev)?.is_consumed() {
				if let Event::Key(k) = ev {
					if key_match(k, self.key_config.keys.enter) {
						self.submit_comment();
					} else if key_match(
						k,
						self.key_config.keys.exit_popup,
					) {
						self.comment_input.hide();
					}
				}
			}
			return Ok(Some(EventState::Consumed));
		}

		if self.label_input.is_visible() {
			if !self.label_input.event(ev)?.is_consumed() {
				if let Event::Key(k) = ev {
					if key_match(k, self.key_config.keys.enter) {
						self.submit_labels();
					} else if key_match(
						k,
						self.key_config.keys.exit_popup,
					) {
						self.label_input.hide();
					}
				}
			}
			return Ok(Some(EventState::Consumed));
		}

		if self.filter_input.is_visible() {
			if !self.filter_input.event(ev)?.is_consumed() {
				if let Event::Key(k) = ev {
					if key_match(k, self.key_config.keys.enter) {
						self.submit_filter();
					} else if key_match(
						k,
						self.key_config.keys.exit_popup,
					) {
						self.filter_input.hide();
					}
				}
			}
			return Ok(Some(EventState::Consumed));
		}

		Ok(None)
	}

	/// Handle a key while the MR detail view is open.
	fn detail_event(&mut self, k: &KeyEvent) {
		if key_match(k, self.key_config.keys.exit_popup) {
			self.close_detail();
		} else if key_match(k, self.key_config.keys.move_down) {
			self.scroll_detail(true);
		} else if key_match(k, self.key_config.keys.move_up) {
			self.scroll_detail(false);
		} else if matches!(k.code, KeyCode::Char('n')) {
			self.show_comment_prompt();
		} else if matches!(k.code, KeyCode::Char('d')) {
			self.open_changes();
		} else if matches!(k.code, KeyCode::Char('l')) {
			self.show_label_prompt();
		} else if matches!(k.code, KeyCode::Char('o')) {
			self.open_in_browser();
		} else if matches!(k.code, KeyCode::Char('m')) {
			if let Some(iid) = self.action_iid() {
				self.spawn_action(
					GitLabAction::MergeMergeRequest { iid },
				);
			}
		} else if matches!(k.code, KeyCode::Char('a')) {
			if let Some(iid) = self.action_iid() {
				self.spawn_action(
					GitLabAction::ApproveMergeRequest { iid },
				);
			}
		} else if matches!(k.code, KeyCode::Char('u')) {
			if let Some(iid) = self.action_iid() {
				self.spawn_action(
					GitLabAction::UnapproveMergeRequest { iid },
				);
			}
		} else if matches!(k.code, KeyCode::Char('b')) {
			if let Some(iid) = self.action_iid() {
				self.spawn_action(
					GitLabAction::RebaseMergeRequest { iid },
				);
			}
		} else if matches!(k.code, KeyCode::Char('c')) {
			self.toggle_state();
		}
	}
}

const fn ci_marker(status: CiStatus) -> &'static str {
	match status {
		CiStatus::Success => "✓",
		CiStatus::Failed => "✗",
		CiStatus::Running => "▶",
		CiStatus::Canceled => "⊘",
		CiStatus::Skipped => "»",
		CiStatus::Manual => "⏸",
		CiStatus::Unknown => "?",
		_ => "•",
	}
}

/// Inspect the default remote and parse it into a GitLab project, if any.
fn detect_gitlab_remote(
	repo: &RepoPathRef,
) -> Option<GitLabRemote> {
	let repo = repo.borrow();
	let remote_name = get_default_remote(&repo).ok()?;
	let url = get_remote_url(&repo, &remote_name).ok()??;
	GitLabRemote::from_url(&url).ok()
}

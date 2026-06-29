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
	has_token, store_token, AsyncActionJob, AsyncBoardJob,
	AsyncGitLabNotification, AsyncIssueDetailJob, AsyncIssuesJob,
	BoardColumn, BoardView, GitLabAction, GitLabRemote, Issue,
	IssueScope, IssueState, Note, StateEvent,
};
use crossterm::event::{Event, KeyCode};
use ratatui::{
	layout::{
		Alignment, Constraint, Direction, Layout, Rect,
	},
	text::{Line, Span},
	widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
	Frame,
};

/// Loaded detail of a single issue: the issue plus its comment thread.
struct DetailData {
	issue: Issue,
	notes: Vec<Note>,
}

/// Loading state of a fetched payload.
enum Load<T> {
	Loading,
	Loaded(T),
	Error(String),
}

/// Which view of the issues is shown.
#[derive(Clone, Copy, PartialEq, Eq)]
enum View {
	List,
	Board,
}

pub struct IssuesTab {
	visible: bool,
	remote: Option<GitLabRemote>,
	view: View,
	list: Load<Vec<Issue>>,
	board: Load<BoardView>,
	/// which board to show (index into the project's boards)
	board_index: usize,
	/// open issue detail view, if any
	detail: Option<Load<DetailData>>,
	/// iid of the issue shown in the detail view (for reloading)
	detail_iid: Option<u64>,
	/// scroll offset of the detail panel
	detail_scroll: u16,
	/// selection in the flat list view
	selection: usize,
	/// active column / row in the board view
	board_col: usize,
	board_row: usize,
	/// case-insensitive substring filter applied to the list view
	filter: String,
	/// transient one-line feedback after a write action
	status_msg: Option<String>,
	/// error from storing a token in the keyring
	token_error: Option<String>,
	async_issues: AsyncSingleJob<AsyncIssuesJob>,
	async_board: AsyncSingleJob<AsyncBoardJob>,
	async_detail: AsyncSingleJob<AsyncIssueDetailJob>,
	async_action: AsyncSingleJob<AsyncActionJob>,
	token_input: TextInputComponent,
	new_issue_input: TextInputComponent,
	comment_input: TextInputComponent,
	label_input: TextInputComponent,
	filter_input: TextInputComponent,
	theme: SharedTheme,
	key_config: SharedKeyConfig,
}

impl IssuesTab {
	pub fn new(env: &Environment) -> Self {
		let remote = detect_gitlab_remote(&env.repo);

		let token_input = TextInputComponent::new(
			env,
			"GitLab token",
			"paste a token with api scope, then press [Enter]",
			false,
		)
		.with_input_type(InputType::Password);

		let new_issue_input = TextInputComponent::new(
			env,
			"New issue",
			"issue title, then press [Enter]",
			false,
		);

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
			view: View::List,
			list: Load::Loading,
			board: Load::Loading,
			board_index: 0,
			detail: None,
			detail_iid: None,
			detail_scroll: 0,
			selection: 0,
			board_col: 0,
			board_row: 0,
			filter: String::new(),
			status_msg: None,
			token_error: None,
			async_issues: AsyncSingleJob::new(
				env.sender_gitlab.clone(),
			),
			async_board: AsyncSingleJob::new(
				env.sender_gitlab.clone(),
			),
			async_detail: AsyncSingleJob::new(
				env.sender_gitlab.clone(),
			),
			async_action: AsyncSingleJob::new(
				env.sender_gitlab.clone(),
			),
			token_input,
			new_issue_input,
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

	/// True when a GitLab remote was detected and a token is available.
	fn token_available(&self) -> bool {
		self.remote
			.as_ref()
			.is_some_and(|r| has_token(&r.host))
	}

	/// Spawn the fetch for the active view if it has not loaded yet.
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

		match self.view {
			View::List => {
				if matches!(self.list, Load::Loading)
					&& !self.async_issues.is_pending()
				{
					self.async_issues.spawn(AsyncIssuesJob::new(
						remote,
						IssueScope::Opened,
					));
				}
			}
			View::Board => {
				if matches!(self.board, Load::Loading)
					&& !self.async_board.is_pending()
				{
					self.async_board.spawn(AsyncBoardJob::new(
						remote,
						self.board_index,
					));
				}
			}
		}
	}

	/// Force a reload of the active view.
	fn reload(&mut self) {
		match self.view {
			View::List => self.list = Load::Loading,
			View::Board => self.board = Load::Loading,
		}
		self.ensure_load();
	}

	fn toggle_view(&mut self) {
		self.view = match self.view {
			View::List => View::Board,
			View::Board => View::List,
		};
		self.ensure_load();
	}

	/// Open the detail view for the currently selected issue.
	fn open_detail(&mut self) {
		let Some(iid) = self.selected_issue().map(|i| i.iid) else {
			return;
		};
		let Some(remote) = self.remote.clone() else {
			return;
		};
		self.detail_iid = Some(iid);
		self.detail_scroll = 0;
		self.detail = Some(Load::Loading);
		self.async_detail
			.spawn(AsyncIssueDetailJob::new(remote, iid));
	}

	fn close_detail(&mut self) {
		self.detail = None;
		self.detail_iid = None;
		self.detail_scroll = 0;
	}

	/// Re-fetch the detail currently being shown (after a comment/close).
	fn reload_detail(&mut self) {
		let (Some(iid), Some(remote)) =
			(self.detail_iid, self.remote.clone())
		else {
			return;
		};
		self.detail = Some(Load::Loading);
		self.async_detail
			.spawn(AsyncIssueDetailJob::new(remote, iid));
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

	/// Post the typed comment to the issue shown in the detail view.
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
		self.spawn_action(GitLabAction::CreateIssueNote {
			iid,
			body,
		});
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
			|| self.new_issue_input.is_visible()
			|| self.comment_input.is_visible()
			|| self.label_input.is_visible()
			|| self.filter_input.is_visible()
	}

	/// Persist the typed token to the OS keyring, then start loading.
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
				self.board = Load::Loading;
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

	fn show_new_issue_prompt(&mut self) {
		if !self.new_issue_input.is_visible() {
			self.new_issue_input.clear();
			let _ = self.new_issue_input.show();
		}
	}

	/// Spawn a "create issue" action with the typed title.
	fn submit_new_issue(&mut self) {
		let title = self.new_issue_input.get_text().trim().to_string();
		self.new_issue_input.clear();
		self.new_issue_input.hide();
		if title.is_empty() {
			return;
		}
		self.spawn_action(GitLabAction::CreateIssue {
			title,
			description: None,
		});
	}

	/// True when an opened (still closable) issue is selected.
	fn selected_is_open(&self) -> bool {
		self.selected_issue()
			.is_some_and(|i| i.state != IssueState::Closed)
	}

	/// Close the selected issue, or reopen it if already closed.
	fn toggle_state_selected(&mut self) {
		let (iid, event) = match self.selected_issue() {
			Some(i) if i.state == IssueState::Closed => {
				(i.iid, StateEvent::Reopen)
			}
			Some(i) => (i.iid, StateEvent::Close),
			None => return,
		};
		self.spawn_action(GitLabAction::SetIssueState {
			iid,
			event,
		});
	}

	/// Open the selected issue (or the one in the detail view) in a browser.
	fn open_in_browser(&mut self) {
		let url = if let Some(Load::Loaded(d)) = &self.detail {
			Some(d.issue.web_url.clone())
		} else {
			self.selected_issue().map(|i| i.web_url.clone())
		};
		let Some(url) = url.filter(|u| !u.is_empty()) else {
			return;
		};
		if let Err(e) = crate::open_browser::open_in_browser(&url) {
			self.status_msg = Some(format!("error: {e}"));
		}
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

	/// The issue actions apply to: the detail's issue, else the selection.
	fn action_issue(&self) -> Option<(u64, Vec<String>)> {
		if let Some(Load::Loaded(d)) = &self.detail {
			return Some((d.issue.iid, d.issue.labels.clone()));
		}
		self.selected_issue().map(|i| (i.iid, i.labels.clone()))
	}

	fn show_label_prompt(&mut self) {
		if self.label_input.is_visible() {
			return;
		}
		let labels = self
			.action_issue()
			.map(|(_, l)| l.join(", "))
			.unwrap_or_default();
		self.label_input.set_text(labels);
		let _ = self.label_input.show();
	}

	fn submit_labels(&mut self) {
		let labels = self.label_input.get_text().trim().to_string();
		self.label_input.hide();
		let Some((iid, _)) = self.action_issue() else {
			return;
		};
		self.spawn_action(GitLabAction::SetIssueLabels {
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
			AsyncGitLabNotification::Issues => {
				if let Some(job) = self.async_issues.take_last() {
					if let Some(result) = job.result() {
						self.list = match result {
							Ok(issues) => Load::Loaded(issues),
							Err(e) => Load::Error(e),
						};
						self.clamp_selection();
					}
				}
			}
			AsyncGitLabNotification::Board => {
				if let Some(job) = self.async_board.take_last() {
					if let Some(result) = job.result() {
						self.board = match result {
							Ok(view) => {
								self.board_index = view.selected;
								Load::Loaded(view)
							}
							Err(e) => Load::Error(e),
						};
						self.clamp_board_selection();
					}
				}
			}
			AsyncGitLabNotification::IssueDetail => {
				if let Some(job) = self.async_detail.take_last() {
					if let Some(result) = job.result() {
						self.detail = Some(match result {
							Ok((issue, notes)) => {
								Load::Loaded(DetailData { issue, notes })
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
			AsyncGitLabNotification::MergeRequests
			| AsyncGitLabNotification::MrDetail
			| AsyncGitLabNotification::MrChanges
			| AsyncGitLabNotification::Pipelines
			| AsyncGitLabNotification::PipelineJobs
			| AsyncGitLabNotification::JobTrace
			| AsyncGitLabNotification::Commits
			| AsyncGitLabNotification::CommitStatuses => {}
		}
	}

	pub fn any_work_pending(&self) -> bool {
		self.async_issues.is_pending()
			|| self.async_board.is_pending()
			|| self.async_detail.is_pending()
			|| self.async_action.is_pending()
	}

	fn list_issues(&self) -> Option<&[Issue]> {
		match &self.list {
			Load::Loaded(issues) => Some(issues),
			_ => None,
		}
	}

	fn matches_filter(&self, issue: &Issue) -> bool {
		if self.filter.is_empty() {
			return true;
		}
		let f = self.filter.to_lowercase();
		issue.title.to_lowercase().contains(&f)
			|| issue
				.author
				.as_ref()
				.is_some_and(|a| {
					a.username.to_lowercase().contains(&f)
				})
			|| issue
				.labels
				.iter()
				.any(|l| l.to_lowercase().contains(&f))
			|| format!("#{}", issue.iid).contains(&f)
	}

	/// Issues shown in the list view (after applying the filter).
	fn filtered_issues(&self) -> Vec<&Issue> {
		self.list_issues().map_or_else(Vec::new, |issues| {
			issues
				.iter()
				.filter(|i| self.matches_filter(i))
				.collect()
		})
	}

	fn board_columns(&self) -> Option<&[BoardColumn]> {
		match &self.board {
			Load::Loaded(view) => Some(&view.columns),
			_ => None,
		}
	}

	/// Switch to the next/previous board, if more than one exists.
	fn cycle_board(&mut self, next: bool) {
		let Load::Loaded(view) = &self.board else {
			return;
		};
		let len = view.board_names.len();
		if len <= 1 {
			return;
		}
		let current = view.selected;
		self.board_index = if next {
			(current + 1) % len
		} else {
			current.checked_sub(1).unwrap_or(len - 1)
		};
		self.board = Load::Loading;
		self.board_col = 0;
		self.board_row = 0;
		self.ensure_load();
	}

	/// True when the active view has finished loading at least once.
	fn content_loaded(&self) -> bool {
		match self.view {
			View::List => self.list_issues().is_some(),
			View::Board => self.board_columns().is_some(),
		}
	}

	fn selected_issue(&self) -> Option<&Issue> {
		match self.view {
			View::List => {
				self.filtered_issues().get(self.selection).copied()
			}
			View::Board => self
				.board_columns()
				.and_then(|c| c.get(self.board_col))
				.and_then(|col| col.issues.get(self.board_row)),
		}
	}

	fn clamp_selection(&mut self) {
		let len = self.filtered_issues().len();
		if len == 0 {
			self.selection = 0;
		} else if self.selection >= len {
			self.selection = len - 1;
		}
	}

	fn clamp_board_selection(&mut self) {
		let cols = self.board_columns().map_or(0, <[_]>::len);
		if cols == 0 {
			self.board_col = 0;
			self.board_row = 0;
			return;
		}
		if self.board_col >= cols {
			self.board_col = cols - 1;
		}
		let rows = self
			.board_columns()
			.and_then(|c| c.get(self.board_col))
			.map_or(0, |c| c.issues.len());
		if rows == 0 {
			self.board_row = 0;
		} else if self.board_row >= rows {
			self.board_row = rows - 1;
		}
	}

	fn move_selection(&mut self, down: bool) {
		let len = self.filtered_issues().len();
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

	fn move_board_col(&mut self, right: bool) {
		let cols = self.board_columns().map_or(0, <[_]>::len);
		if cols == 0 {
			return;
		}
		if right {
			self.board_col = (self.board_col + 1) % cols;
		} else {
			self.board_col =
				self.board_col.checked_sub(1).unwrap_or(cols - 1);
		}
		self.board_row = 0;
	}

	fn move_board_row(&mut self, down: bool) {
		let rows = self
			.board_columns()
			.and_then(|c| c.get(self.board_col))
			.map_or(0, |c| c.issues.len());
		if rows == 0 {
			return;
		}
		if down {
			self.board_row = (self.board_row + 1) % rows;
		} else {
			self.board_row =
				self.board_row.checked_sub(1).unwrap_or(rows - 1);
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
		let view = match self.view {
			View::List => "list",
			View::Board => "board",
		};
		self.remote.as_ref().map_or_else(
			|| format!("Issues ({view})"),
			|r| format!("Issues · {} ({view})", r.project_path),
		)
	}

	fn host(&self) -> &str {
		self.remote.as_ref().map_or("", |r| r.host.as_str())
	}

	/// Split off a one-line footer for transient action feedback.
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
			let p = Paragraph::new(msg)
				.style(self.theme.text(true, false));
			f.render_widget(p, rect);
		}
	}

	fn issue_line(&self, issue: &Issue, selected: bool) -> Line<'_> {
		let marker = match issue.state {
			IssueState::Closed => "✗",
			_ => "●",
		};
		let author = issue
			.author
			.as_ref()
			.map_or_else(String::new, |a| format!(" @{}", a.username));
		let comments = if issue.user_notes_count > 0 {
			format!("  💬{}", issue.user_notes_count)
		} else {
			String::new()
		};
		Line::from(vec![Span::styled(
			format!(
				"{marker} #{}  {}{author}{comments}",
				issue.iid, issue.title,
			),
			self.theme.text(true, selected),
		)])
	}

	fn render_list(&self, f: &mut Frame, rect: Rect) {
		let (list_area, footer) = self.split_footer(rect);
		let issues = self.filtered_issues();

		let items: Vec<ListItem> = issues
			.iter()
			.enumerate()
			.map(|(i, issue)| {
				ListItem::new(
					self.issue_line(issue, i == self.selection),
				)
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

	fn render_board(
		&self,
		f: &mut Frame,
		rect: Rect,
		view: &BoardView,
	) {
		let (content, footer) = self.split_footer(rect);

		// one-line header: board name + switch hint
		let name = view
			.board_names
			.get(view.selected)
			.map_or("board", String::as_str);
		let switch = if view.board_names.len() > 1 {
			format!(
				"   ([/] switch · {}/{})",
				view.selected + 1,
				view.board_names.len()
			)
		} else {
			String::new()
		};
		let chunks = Layout::default()
			.direction(Direction::Vertical)
			.constraints([
				Constraint::Length(1),
				Constraint::Min(1),
			])
			.split(content);
		f.render_widget(
			Paragraph::new(format!("Board: {name}{switch}"))
				.style(self.theme.text(true, true)),
			chunks[0],
		);
		let board_area = chunks[1];

		let columns = &view.columns;
		if columns.is_empty() {
			self.draw_message(f, board_area, "No board columns.");
			self.draw_footer(f, footer);
			return;
		}

		let col_count = u32::try_from(columns.len()).unwrap_or(1);
		let constraints: Vec<Constraint> = columns
			.iter()
			.map(|_| Constraint::Ratio(1, col_count))
			.collect();
		let areas = Layout::default()
			.direction(Direction::Horizontal)
			.constraints(constraints)
			.split(board_area);

		for (ci, (col, area)) in
			columns.iter().zip(areas.iter()).enumerate()
		{
			let active_col = ci == self.board_col;
			let items: Vec<ListItem> = col
				.issues
				.iter()
				.enumerate()
				.map(|(ri, issue)| {
					let selected =
						active_col && ri == self.board_row;
					ListItem::new(self.issue_line(issue, selected))
				})
				.collect();

			let title = format!("{} ({})", col.title, col.issues.len());
			let block = Block::default()
				.borders(Borders::ALL)
				.title(Span::styled(
					title,
					self.theme.text(true, active_col),
				));
			f.render_widget(List::new(items).block(block), *area);
		}

		self.draw_footer(f, footer);
	}

	fn render_detail(
		&self,
		f: &mut Frame,
		rect: Rect,
		data: &DetailData,
	) {
		let (area, footer) = self.split_footer(rect);
		let style = self.theme.text(true, false);
		let header = self.theme.text(true, true);
		let issue = &data.issue;

		let state = match issue.state {
			IssueState::Closed => "closed",
			_ => "open",
		};
		let author = issue
			.author
			.as_ref()
			.map_or_else(String::new, |a| {
				format!("  by @{}", a.username)
			});

		let mut lines: Vec<Line> = Vec::new();
		lines.push(Line::styled(
			format!("#{}  {}", issue.iid, issue.title),
			header,
		));
		lines.push(Line::styled(
			format!("[{state}]{author}   👍{}", issue.upvotes),
			style,
		));
		if !issue.labels.is_empty() {
			lines.push(Line::styled(
				format!("labels: {}", issue.labels.join(", ")),
				style,
			));
		}
		lines.push(Line::raw(""));
		match issue
			.description
			.as_deref()
			.filter(|d| !d.trim().is_empty())
		{
			Some(desc) => {
				for l in desc.lines() {
					lines.push(Line::styled(l.to_string(), style));
				}
			}
			None => lines
				.push(Line::styled("(no description)", style)),
		}

		let comments: Vec<&Note> =
			data.notes.iter().filter(|n| !n.system).collect();
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

		let close_hint = if issue.state == IssueState::Closed {
			"[c] reopen"
		} else {
			"[c] close"
		};
		let title = format!(
			"Issue #{}  ·  [esc] back  [n] comment  [l] labels  {close_hint}  [o] open",
			issue.iid
		);
		let block = Block::default()
			.borders(Borders::ALL)
			.title(title);
		let p = Paragraph::new(lines)
			.block(block)
			.wrap(Wrap { trim: false })
			.scroll((self.detail_scroll, 0));
		f.render_widget(p, area);
		self.draw_footer(f, footer);
	}
}

impl DrawableComponent for IssuesTab {
	fn draw(&self, f: &mut Frame, rect: Rect) -> Result<()> {
		// remote / token gating, shared by both views
		if self.draw_gate(f, rect)? {
			return Ok(());
		}

		// detail view takes over the whole area when open
		if let Some(detail) = &self.detail {
			match detail {
				Load::Loading => {
					self.draw_message(f, rect, "Loading issue…");
				}
				Load::Error(e) => self.draw_message(
					f,
					rect,
					&format!(
						"Failed to load issue:\n{e}\n\nPress [Esc] to go back."
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

		// token available: render the active view's content
		let content = match self.view {
			View::List => &self.list as &dyn LoadStatus,
			View::Board => &self.board as &dyn LoadStatus,
		};
		match content.status() {
			Status::Loading => self.draw_message(
				f,
				rect,
				match self.view {
					View::List => "Loading issues…",
					View::Board => "Loading board…",
				},
			),
			Status::Error(e) => self.draw_message(
				f,
				rect,
				&format!("Failed to load:\n{e}\n\nPress [r] to retry."),
			),
			Status::Loaded => match self.view {
				View::List => {
					if self.filtered_issues().is_empty() {
						let msg = if self.filter.is_empty() {
							"No open issues.\n\nPress [n] to create one, [b] for board view, [f] to filter."
						} else {
							"No matching issues.\n\nPress [f] to change the filter."
						};
						self.draw_message(f, rect, msg);
					} else {
						self.render_list(f, rect);
					}
				}
				View::Board => {
					if let Load::Loaded(view) = &self.board {
						self.render_board(f, rect, view);
					}
				}
			},
		}

		if self.new_issue_input.is_visible() {
			self.new_issue_input.draw(f, rect)?;
		}
		if self.filter_input.is_visible() {
			self.filter_input.draw(f, rect)?;
		}

		Ok(())
	}
}

/// Tiny view-agnostic adaptor over `Load<T>` for the shared draw branch.
enum Status<'a> {
	Loading,
	Loaded,
	Error(&'a str),
}
trait LoadStatus {
	fn status(&self) -> Status<'_>;
}
impl<T> LoadStatus for Load<T> {
	fn status(&self) -> Status<'_> {
		match self {
			Self::Loading => Status::Loading,
			Self::Loaded(_) => Status::Loaded,
			Self::Error(e) => Status::Error(e),
		}
	}
}

impl Component for IssuesTab {
	fn commands(
		&self,
		out: &mut Vec<CommandInfo>,
		force_all: bool,
	) -> CommandBlocking {
		if self.visible || force_all {
			out.push(CommandInfo::new(
				strings::commands::scroll(&self.key_config),
				self.content_loaded(),
				true,
			));
			out.push(CommandInfo::new(
				strings::commands::issue_open(&self.key_config),
				self.selected_issue().is_some(),
				self.content_loaded(),
			));
			out.push(CommandInfo::new(
				strings::commands::issue_board(&self.key_config),
				true,
				true,
			));
			out.push(CommandInfo::new(
				strings::commands::issue_new(&self.key_config),
				true,
				self.content_loaded(),
			));
			let close_cmd = if self.selected_is_open() {
				strings::commands::issue_close(&self.key_config)
			} else {
				strings::commands::issue_reopen(&self.key_config)
			};
			out.push(CommandInfo::new(
				close_cmd,
				self.selected_issue().is_some(),
				self.content_loaded(),
			));
			out.push(CommandInfo::new(
				strings::commands::gitlab_browser(&self.key_config),
				self.selected_issue().is_some() || self.detail_open(),
				self.content_loaded() || self.detail_open(),
			));
		}

		visibility_blocking(self)
	}

	fn event(&mut self, ev: &Event) -> Result<EventState> {
		if !self.is_visible() {
			return Ok(EventState::NotConsumed);
		}

		// an active text input owns all keys
		if let Some(state) = self.input_event(ev)? {
			return Ok(state);
		}

		// the detail view owns navigation while open
		if self.detail_open() {
			if let Event::Key(k) = ev {
				self.detail_event(k);
			}
			return Ok(EventState::Consumed);
		}

		if let Event::Key(k) = ev {
			let token_missing = !self.token_available();

			if key_match(k, self.key_config.keys.move_down) {
				match self.view {
					View::List => self.move_selection(true),
					View::Board => self.move_board_row(true),
				}
				return Ok(EventState::Consumed);
			} else if key_match(k, self.key_config.keys.move_up) {
				match self.view {
					View::List => self.move_selection(false),
					View::Board => self.move_board_row(false),
				}
				return Ok(EventState::Consumed);
			} else if key_match(k, self.key_config.keys.move_right)
				&& self.view == View::Board
			{
				self.move_board_col(true);
				return Ok(EventState::Consumed);
			} else if key_match(k, self.key_config.keys.move_left)
				&& self.view == View::Board
			{
				self.move_board_col(false);
				return Ok(EventState::Consumed);
			} else if matches!(k.code, KeyCode::Char(']'))
				&& self.view == View::Board
			{
				self.cycle_board(true);
				return Ok(EventState::Consumed);
			} else if matches!(k.code, KeyCode::Char('['))
				&& self.view == View::Board
			{
				self.cycle_board(false);
				return Ok(EventState::Consumed);
			} else if key_match(k, self.key_config.keys.enter) {
				if token_missing {
					self.show_token_prompt();
				} else if self.selected_issue().is_some() {
					self.open_detail();
				}
				return Ok(EventState::Consumed);
			} else if matches!(k.code, KeyCode::Char('b'))
				&& !token_missing
			{
				self.toggle_view();
				return Ok(EventState::Consumed);
			} else if matches!(k.code, KeyCode::Char('n'))
				&& self.content_loaded()
			{
				self.show_new_issue_prompt();
				return Ok(EventState::Consumed);
			} else if matches!(k.code, KeyCode::Char('c'))
				&& self.selected_issue().is_some()
			{
				self.toggle_state_selected();
				return Ok(EventState::Consumed);
			} else if matches!(k.code, KeyCode::Char('o'))
				&& self.selected_issue().is_some()
			{
				self.open_in_browser();
				return Ok(EventState::Consumed);
			} else if matches!(k.code, KeyCode::Char('l'))
				&& self.selected_issue().is_some()
			{
				self.show_label_prompt();
				return Ok(EventState::Consumed);
			} else if matches!(k.code, KeyCode::Char('f'))
				&& !token_missing
				&& self.view == View::List
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

impl IssuesTab {
	/// Draw the no-remote / token-required screens. Returns `true` when it
	/// handled drawing and the caller should stop.
	fn draw_gate(&self, f: &mut Frame, rect: Rect) -> Result<bool> {
		if self.remote.is_none() {
			self.draw_message(
				f,
				rect,
				"No GitLab remote detected for this repository.",
			);
			return Ok(true);
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
					&format!("{err}\n\nPress [Enter] to try again."),
				);
			} else {
				self.draw_message(
					f,
					rect,
					&strings::gitlab_token_help(self.host(), true),
				);
			}
			return Ok(true);
		}
		Ok(false)
	}

	/// Route a key event to whichever text input is active. Returns
	/// `Some(Consumed)` when an input handled it, `None` when none is open.
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

		if self.new_issue_input.is_visible() {
			if !self.new_issue_input.event(ev)?.is_consumed() {
				if let Event::Key(k) = ev {
					if key_match(k, self.key_config.keys.enter) {
						self.submit_new_issue();
					} else if key_match(
						k,
						self.key_config.keys.exit_popup,
					) {
						self.new_issue_input.hide();
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

	/// Handle a key while the issue detail view is open.
	fn detail_event(&mut self, k: &crossterm::event::KeyEvent) {
		if key_match(k, self.key_config.keys.exit_popup) {
			self.close_detail();
		} else if key_match(k, self.key_config.keys.move_down) {
			self.scroll_detail(true);
		} else if key_match(k, self.key_config.keys.move_up) {
			self.scroll_detail(false);
		} else if matches!(k.code, KeyCode::Char('n')) {
			self.show_comment_prompt();
		} else if matches!(k.code, KeyCode::Char('l')) {
			self.show_label_prompt();
		} else if matches!(k.code, KeyCode::Char('o')) {
			self.open_in_browser();
		} else if matches!(k.code, KeyCode::Char('c')) {
			let action = match &self.detail {
				Some(Load::Loaded(d)) => Some((
					d.issue.iid,
					if d.issue.state == IssueState::Closed {
						StateEvent::Reopen
					} else {
						StateEvent::Close
					},
				)),
				_ => None,
			};
			if let Some((iid, event)) = action {
				self.spawn_action(GitLabAction::SetIssueState {
					iid,
					event,
				});
			}
		}
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

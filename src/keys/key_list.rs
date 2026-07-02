use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde::{Deserialize, Serialize};
use std::{fs::File, path::PathBuf};
use struct_patch::traits::Patch as PatchTrait;
use struct_patch::Patch;

#[derive(Debug, PartialOrd, Clone, Copy, Serialize, Deserialize)]
pub struct LabtuiKeyEvent {
	pub code: KeyCode,
	pub modifiers: KeyModifiers,
}

impl LabtuiKeyEvent {
	pub const fn new(code: KeyCode, modifiers: KeyModifiers) -> Self {
		Self { code, modifiers }
	}
}

pub fn key_match(ev: &KeyEvent, binding: LabtuiKeyEvent) -> bool {
	ev.code == binding.code && ev.modifiers == binding.modifiers
}

impl PartialEq for LabtuiKeyEvent {
	fn eq(&self, other: &Self) -> bool {
		let ev: KeyEvent = self.into();
		let other: KeyEvent = other.into();
		ev == other
	}
}

impl From<&LabtuiKeyEvent> for KeyEvent {
	fn from(other: &LabtuiKeyEvent) -> Self {
		Self::new(other.code, other.modifiers)
	}
}

#[derive(Debug, Clone, Patch)]
#[patch(attribute(derive(Deserialize, Debug)))]
pub struct KeysList {
	pub tab_status: LabtuiKeyEvent,
	pub tab_log: LabtuiKeyEvent,
	pub tab_files: LabtuiKeyEvent,
	pub tab_stashing: LabtuiKeyEvent,
	pub tab_stashes: LabtuiKeyEvent,
	pub tab_merge_requests: LabtuiKeyEvent,
	pub tab_issues: LabtuiKeyEvent,
	pub tab_pipelines: LabtuiKeyEvent,
	pub tab_toggle: LabtuiKeyEvent,
	pub tab_toggle_reverse: LabtuiKeyEvent,
	pub toggle_workarea: LabtuiKeyEvent,
	pub exit: LabtuiKeyEvent,
	pub quit: LabtuiKeyEvent,
	pub exit_popup: LabtuiKeyEvent,
	pub open_commit: LabtuiKeyEvent,
	pub open_commit_editor: LabtuiKeyEvent,
	pub open_help: LabtuiKeyEvent,
	pub open_options: LabtuiKeyEvent,
	pub move_left: LabtuiKeyEvent,
	pub move_right: LabtuiKeyEvent,
	pub move_up: LabtuiKeyEvent,
	pub move_down: LabtuiKeyEvent,
	pub tree_collapse_recursive: LabtuiKeyEvent,
	pub tree_expand_recursive: LabtuiKeyEvent,
	pub home: LabtuiKeyEvent,
	pub end: LabtuiKeyEvent,
	pub popup_up: LabtuiKeyEvent,
	pub popup_down: LabtuiKeyEvent,
	pub page_down: LabtuiKeyEvent,
	pub page_up: LabtuiKeyEvent,
	pub shift_up: LabtuiKeyEvent,
	pub shift_down: LabtuiKeyEvent,
	pub enter: LabtuiKeyEvent,
	pub blame: LabtuiKeyEvent,
	pub file_history: LabtuiKeyEvent,
	pub edit_file: LabtuiKeyEvent,
	pub status_stage_all: LabtuiKeyEvent,
	pub status_reset_item: LabtuiKeyEvent,
	pub status_ignore_file: LabtuiKeyEvent,
	pub diff_stage_lines: LabtuiKeyEvent,
	pub diff_reset_lines: LabtuiKeyEvent,
	pub stashing_save: LabtuiKeyEvent,
	pub stashing_toggle_untracked: LabtuiKeyEvent,
	pub stashing_toggle_index: LabtuiKeyEvent,
	pub stash_apply: LabtuiKeyEvent,
	pub stash_open: LabtuiKeyEvent,
	pub stash_drop: LabtuiKeyEvent,
	pub cmd_bar_toggle: LabtuiKeyEvent,
	pub log_tag_commit: LabtuiKeyEvent,
	pub log_mark_commit: LabtuiKeyEvent,
	pub log_checkout_commit: LabtuiKeyEvent,
	pub log_reset_commit: LabtuiKeyEvent,
	pub log_reword_commit: LabtuiKeyEvent,
	pub log_find: LabtuiKeyEvent,
	pub find_commit_sha: LabtuiKeyEvent,
	pub commit_amend: LabtuiKeyEvent,
	pub toggle_signoff: LabtuiKeyEvent,
	pub toggle_verify: LabtuiKeyEvent,
	pub copy: LabtuiKeyEvent,
	pub create_branch: LabtuiKeyEvent,
	pub rename_branch: LabtuiKeyEvent,
	pub select_branch: LabtuiKeyEvent,
	pub delete_branch: LabtuiKeyEvent,
	pub merge_branch: LabtuiKeyEvent,
	pub rebase_branch: LabtuiKeyEvent,
	pub reset_branch: LabtuiKeyEvent,
	pub compare_commits: LabtuiKeyEvent,
	pub tags: LabtuiKeyEvent,
	pub delete_tag: LabtuiKeyEvent,
	pub select_tag: LabtuiKeyEvent,
	pub push: LabtuiKeyEvent,
	pub open_file_tree: LabtuiKeyEvent,
	pub file_find: LabtuiKeyEvent,
	pub branch_find: LabtuiKeyEvent,
	pub force_push: LabtuiKeyEvent,
	pub fetch: LabtuiKeyEvent,
	pub pull: LabtuiKeyEvent,
	pub abort_merge: LabtuiKeyEvent,
	pub undo_commit: LabtuiKeyEvent,
	pub diff_hunk_next: LabtuiKeyEvent,
	pub diff_hunk_prev: LabtuiKeyEvent,
	pub stage_unstage_item: LabtuiKeyEvent,
	pub tag_annotate: LabtuiKeyEvent,
	pub view_submodules: LabtuiKeyEvent,
	pub view_remotes: LabtuiKeyEvent,
	pub update_remote_name: LabtuiKeyEvent,
	pub update_remote_url: LabtuiKeyEvent,
	pub add_remote: LabtuiKeyEvent,
	pub delete_remote: LabtuiKeyEvent,
	pub view_submodule_parent: LabtuiKeyEvent,
	pub update_submodule: LabtuiKeyEvent,
	pub commit_history_next: LabtuiKeyEvent,
	pub commit: LabtuiKeyEvent,
	pub newline: LabtuiKeyEvent,
	pub goto_line: LabtuiKeyEvent,
}

#[rustfmt::skip]
impl Default for KeysList {
	fn default() -> Self {
		Self {
			tab_status: LabtuiKeyEvent::new(KeyCode::Char('1'), KeyModifiers::empty()),
			tab_log: LabtuiKeyEvent::new(KeyCode::Char('2'),  KeyModifiers::empty()),
			tab_files: LabtuiKeyEvent::new(KeyCode::Char('3'),  KeyModifiers::empty()),
			tab_stashing: LabtuiKeyEvent::new(KeyCode::Char('4'),  KeyModifiers::empty()),
			tab_stashes: LabtuiKeyEvent::new(KeyCode::Char('5'),  KeyModifiers::empty()),
			tab_merge_requests: LabtuiKeyEvent::new(KeyCode::Char('6'),  KeyModifiers::empty()),
			tab_issues: LabtuiKeyEvent::new(KeyCode::Char('7'),  KeyModifiers::empty()),
			tab_pipelines: LabtuiKeyEvent::new(KeyCode::Char('8'),  KeyModifiers::empty()),
			tab_toggle: LabtuiKeyEvent::new(KeyCode::Tab,  KeyModifiers::empty()),
			tab_toggle_reverse: LabtuiKeyEvent::new(KeyCode::BackTab,  KeyModifiers::SHIFT),
			toggle_workarea: LabtuiKeyEvent::new(KeyCode::Char('w'),  KeyModifiers::empty()),
			exit: LabtuiKeyEvent::new(KeyCode::Char('c'),  KeyModifiers::CONTROL),
			quit: LabtuiKeyEvent::new(KeyCode::Char('q'),  KeyModifiers::empty()),
			exit_popup: LabtuiKeyEvent::new(KeyCode::Esc,  KeyModifiers::empty()),
			open_commit: LabtuiKeyEvent::new(KeyCode::Char('c'),  KeyModifiers::empty()),
			open_commit_editor: LabtuiKeyEvent::new(KeyCode::Char('e'), KeyModifiers::CONTROL),
			open_help: LabtuiKeyEvent::new(KeyCode::Char('h'),  KeyModifiers::empty()),
			open_options: LabtuiKeyEvent::new(KeyCode::Char('o'),  KeyModifiers::empty()),
			move_left: LabtuiKeyEvent::new(KeyCode::Left,  KeyModifiers::empty()),
			move_right: LabtuiKeyEvent::new(KeyCode::Right,  KeyModifiers::empty()),
			tree_collapse_recursive: LabtuiKeyEvent::new(KeyCode::Left,  KeyModifiers::SHIFT),
			tree_expand_recursive: LabtuiKeyEvent::new(KeyCode::Right,  KeyModifiers::SHIFT),
			home: LabtuiKeyEvent::new(KeyCode::Home,  KeyModifiers::empty()),
			end: LabtuiKeyEvent::new(KeyCode::End,  KeyModifiers::empty()),
			move_up: LabtuiKeyEvent::new(KeyCode::Up,  KeyModifiers::empty()),
			move_down: LabtuiKeyEvent::new(KeyCode::Down,  KeyModifiers::empty()),
			popup_up: LabtuiKeyEvent::new(KeyCode::Up,  KeyModifiers::empty()),
			popup_down: LabtuiKeyEvent::new(KeyCode::Down,  KeyModifiers::empty()),
			page_down: LabtuiKeyEvent::new(KeyCode::PageDown,  KeyModifiers::empty()),
			page_up: LabtuiKeyEvent::new(KeyCode::PageUp,  KeyModifiers::empty()),
			shift_up: LabtuiKeyEvent::new(KeyCode::Up,  KeyModifiers::SHIFT),
			shift_down: LabtuiKeyEvent::new(KeyCode::Down,  KeyModifiers::SHIFT),
			enter: LabtuiKeyEvent::new(KeyCode::Enter,  KeyModifiers::empty()),
			blame: LabtuiKeyEvent::new(KeyCode::Char('B'),  KeyModifiers::SHIFT),
			file_history: LabtuiKeyEvent::new(KeyCode::Char('H'),  KeyModifiers::SHIFT),
			edit_file: LabtuiKeyEvent::new(KeyCode::Char('e'),  KeyModifiers::empty()),
			status_stage_all: LabtuiKeyEvent::new(KeyCode::Char('a'),  KeyModifiers::empty()),
			status_reset_item: LabtuiKeyEvent::new(KeyCode::Char('D'),  KeyModifiers::SHIFT),
			diff_reset_lines: LabtuiKeyEvent::new(KeyCode::Char('d'),  KeyModifiers::empty()),
			status_ignore_file: LabtuiKeyEvent::new(KeyCode::Char('i'),  KeyModifiers::empty()),
			diff_stage_lines: LabtuiKeyEvent::new(KeyCode::Char('s'),  KeyModifiers::empty()),
			stashing_save: LabtuiKeyEvent::new(KeyCode::Char('s'),  KeyModifiers::empty()),
			stashing_toggle_untracked: LabtuiKeyEvent::new(KeyCode::Char('u'),  KeyModifiers::empty()),
			stashing_toggle_index: LabtuiKeyEvent::new(KeyCode::Char('i'),  KeyModifiers::empty()),
			stash_apply: LabtuiKeyEvent::new(KeyCode::Char('a'),  KeyModifiers::empty()),
			stash_open: LabtuiKeyEvent::new(KeyCode::Right,  KeyModifiers::empty()),
			stash_drop: LabtuiKeyEvent::new(KeyCode::Char('D'),  KeyModifiers::SHIFT),
			cmd_bar_toggle: LabtuiKeyEvent::new(KeyCode::Char('.'),  KeyModifiers::empty()),
			log_tag_commit: LabtuiKeyEvent::new(KeyCode::Char('t'),  KeyModifiers::empty()),
			log_mark_commit: LabtuiKeyEvent::new(KeyCode::Char(' '),  KeyModifiers::empty()),
			log_checkout_commit: LabtuiKeyEvent { code: KeyCode::Char('S'), modifiers: KeyModifiers::SHIFT },
			log_reset_commit: LabtuiKeyEvent { code: KeyCode::Char('R'), modifiers: KeyModifiers::SHIFT },
			log_reword_commit: LabtuiKeyEvent { code: KeyCode::Char('r'), modifiers: KeyModifiers::empty() },
			log_find: LabtuiKeyEvent { code: KeyCode::Char('f'), modifiers: KeyModifiers::empty() },
			find_commit_sha: LabtuiKeyEvent::new(KeyCode::Char('j'), KeyModifiers::CONTROL),
			commit_amend: LabtuiKeyEvent::new(KeyCode::Char('a'),  KeyModifiers::CONTROL),
			toggle_signoff: LabtuiKeyEvent::new(KeyCode::Char('s'),  KeyModifiers::CONTROL),
			toggle_verify: LabtuiKeyEvent::new(KeyCode::Char('f'),  KeyModifiers::CONTROL),
			copy: LabtuiKeyEvent::new(KeyCode::Char('y'),  KeyModifiers::empty()),
			create_branch: LabtuiKeyEvent::new(KeyCode::Char('c'),  KeyModifiers::empty()),
			rename_branch: LabtuiKeyEvent::new(KeyCode::Char('r'),  KeyModifiers::empty()),
			select_branch: LabtuiKeyEvent::new(KeyCode::Char('b'),  KeyModifiers::empty()),
			delete_branch: LabtuiKeyEvent::new(KeyCode::Char('D'),  KeyModifiers::SHIFT),
			merge_branch: LabtuiKeyEvent::new(KeyCode::Char('m'),  KeyModifiers::empty()),
			rebase_branch: LabtuiKeyEvent::new(KeyCode::Char('R'),  KeyModifiers::SHIFT),
			reset_branch: LabtuiKeyEvent::new(KeyCode::Char('s'),  KeyModifiers::empty()),
			compare_commits: LabtuiKeyEvent::new(KeyCode::Char('C'),  KeyModifiers::SHIFT),
			tags: LabtuiKeyEvent::new(KeyCode::Char('T'),  KeyModifiers::SHIFT),
			delete_tag: LabtuiKeyEvent::new(KeyCode::Char('D'),  KeyModifiers::SHIFT),
			select_tag: LabtuiKeyEvent::new(KeyCode::Enter,  KeyModifiers::empty()),
			push: LabtuiKeyEvent::new(KeyCode::Char('p'),  KeyModifiers::empty()),
			force_push: LabtuiKeyEvent::new(KeyCode::Char('P'),  KeyModifiers::SHIFT),
			undo_commit: LabtuiKeyEvent::new(KeyCode::Char('U'),  KeyModifiers::SHIFT),
			fetch: LabtuiKeyEvent::new(KeyCode::Char('F'),  KeyModifiers::SHIFT),
			pull: LabtuiKeyEvent::new(KeyCode::Char('f'),  KeyModifiers::empty()),
			abort_merge: LabtuiKeyEvent::new(KeyCode::Char('A'),  KeyModifiers::SHIFT),
			open_file_tree: LabtuiKeyEvent::new(KeyCode::Char('F'),  KeyModifiers::SHIFT),
			file_find: LabtuiKeyEvent::new(KeyCode::Char('f'),  KeyModifiers::empty()),
			branch_find: LabtuiKeyEvent::new(KeyCode::Char('f'),  KeyModifiers::empty()),
			diff_hunk_next: LabtuiKeyEvent::new(KeyCode::Char('n'),  KeyModifiers::empty()),
			diff_hunk_prev: LabtuiKeyEvent::new(KeyCode::Char('p'),  KeyModifiers::empty()),
			stage_unstage_item: LabtuiKeyEvent::new(KeyCode::Enter,  KeyModifiers::empty()),
			tag_annotate: LabtuiKeyEvent::new(KeyCode::Char('a'),  KeyModifiers::CONTROL),
			view_submodules: LabtuiKeyEvent::new(KeyCode::Char('S'),  KeyModifiers::SHIFT),
			view_remotes: LabtuiKeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL),
			update_remote_name: LabtuiKeyEvent::new(KeyCode::Char('n'),KeyModifiers::NONE),
			update_remote_url: LabtuiKeyEvent::new(KeyCode::Char('u'),KeyModifiers::NONE),
			add_remote: LabtuiKeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE),
			delete_remote: LabtuiKeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE),
			view_submodule_parent: LabtuiKeyEvent::new(KeyCode::Char('p'),  KeyModifiers::empty()),
			update_submodule: LabtuiKeyEvent::new(KeyCode::Char('u'),  KeyModifiers::empty()),
			commit_history_next: LabtuiKeyEvent::new(KeyCode::Char('n'),  KeyModifiers::CONTROL),
			commit: LabtuiKeyEvent::new(KeyCode::Char('d'),  KeyModifiers::CONTROL),
			newline: LabtuiKeyEvent::new(KeyCode::Enter,  KeyModifiers::empty()),
			goto_line: LabtuiKeyEvent::new(KeyCode::Char('L'),  KeyModifiers::SHIFT),
		}
	}
}

impl KeysList {
	pub fn init(file: PathBuf) -> Self {
		let mut keys_list = Self::default();
		if let Ok(f) = File::open(file) {
			match ron::de::from_reader(f) {
				Ok(patch) => keys_list.apply(patch),
				Err(e) => {
					log::error!("KeysList parse error: {e}");
				}
			}
		}
		keys_list
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use pretty_assertions::assert_eq;
	use std::io::Write;
	use tempfile::NamedTempFile;

	#[test]
	fn test_apply_vim_style_example() {
		let mut keys_list = KeysList::default();
		let f = File::open("vim_style_key_config.ron")
			.expect("vim style config should exist");
		let patch = ron::de::from_reader(f)
			.expect("vim style config format incorrect");
		keys_list.apply(patch);
	}

	#[test]
	fn test_smoke() {
		let mut file = NamedTempFile::new().unwrap();

		writeln!(
			file,
			r#"
(
	move_down: Some(( code: Char('j'), modifiers: "CONTROL")),
	move_up: Some((code: Char('h'), modifiers: ""))
)
"#
		)
		.unwrap();

		let keys = KeysList::init(file.path().to_path_buf());

		assert_eq!(keys.move_right, KeysList::default().move_right);
		assert_eq!(
			keys.move_down,
			LabtuiKeyEvent::new(
				KeyCode::Char('j'),
				KeyModifiers::CONTROL
			)
		);
		assert_eq!(
			keys.move_up,
			LabtuiKeyEvent::new(
				KeyCode::Char('h'),
				KeyModifiers::NONE
			)
		);
	}
}

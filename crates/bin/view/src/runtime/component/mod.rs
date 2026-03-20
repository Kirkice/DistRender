mod components;
mod game_object;
mod scene_state;
mod transform;

pub use components::*;
pub use game_object::*;
pub use scene_state::*;
pub use transform::*;

fn sanitize_ascii_label(name: &str) -> String {
	let mut sanitized = String::with_capacity(name.len());
	let mut previous_was_separator = true;

	for ch in name.chars() {
		let mapped = if ch.is_ascii_alphanumeric() {
			Some(ch)
		} else if ch.is_ascii_whitespace() || matches!(ch, '-' | '_' | '.' | '(' | ')' | '[' | ']') {
			Some(' ')
		} else {
			None
		};

		match mapped {
			Some(' ') if !previous_was_separator => {
				sanitized.push(' ');
				previous_was_separator = true;
			}
			Some(' ') => {}
			Some(value) => {
				sanitized.push(value);
				previous_was_separator = false;
			}
			None => {}
		}
	}

	sanitized.trim().to_owned()
}
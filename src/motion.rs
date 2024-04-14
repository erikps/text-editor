use crate::buffer::{Buffer, Cursor, cursor_add};
use ropey::iter::Chars;

#[derive(Debug, Clone)]
pub enum Motion {
    Left,
    Right,
    Up,
    Down,
    ForwardWord,
    ForwardWordEnd,
    BackWord,
    EndOfLine,
}
fn skip_while<F>(chars: Chars, predicate: F) -> Cursor
where
    F: Fn(usize, char) -> bool,
{
    let mut index = 0;
    for (i, character) in chars.enumerate() {
        if !predicate(i, character) {
            break;
        }
        index += 1;
    }
    index
}


impl Motion {
    /// Return the target location of this movement
    pub fn get_target(self, buffer: &Buffer) -> Cursor {
        match self {
            Motion::ForwardWord => {
                let chars = buffer.text.chars_at(buffer.cursor);
                let is_alphanumeric_start = buffer.text.char(buffer.cursor).is_alphanumeric();
                let offset = skip_while(chars, |_, character| {
                    // skip to the next non-alphanumeric character
                    is_alphanumeric_start == character.is_alphanumeric()
                });
                buffer.get_movement_x(buffer.cursor, offset as i32)
            }
            Motion::ForwardWordEnd => {
                let chars = buffer.text.chars_at(buffer.get_movement_x(buffer.cursor, 1));
                let is_alphanumeric_start = buffer
                    .text
                    .char((buffer.cursor.max(1) + 1).min(buffer.text.len_chars() - 1))
                    .is_alphanumeric();

                let offset = skip_while(chars, |_, character| {
                    // skip to the next non-alphanumeric character
                    is_alphanumeric_start == character.is_alphanumeric()
                }) + 1;
                buffer.get_movement_x(buffer.cursor, offset as i32 - 1)
            }
            Motion::BackWord => {
                let chars = buffer.text.chars_at(buffer.cursor).reversed();
                let is_alphanumeric_start =
                    buffer.text.char(buffer.cursor.max(1) - 1).is_alphanumeric();
                let offset = skip_while(chars, |_, character| {
                    // skip to the next non-alphanumeric character
                    is_alphanumeric_start == character.is_alphanumeric()
                });
                cursor_add(buffer.cursor, -(offset as i32))
            }
            Motion::Left => buffer.get_movement_x(buffer.cursor, -1),
            Motion::Down => buffer.get_movement_y(buffer.cursor, 1),
            Motion::Up => buffer.get_movement_y(buffer.cursor, -1),
            Motion::Right => buffer.get_movement_x(buffer.cursor, 1),

            Motion::EndOfLine => buffer.get_end_of_line_cursor(buffer.cursor),

            _ => buffer.cursor,
        }
    }
}

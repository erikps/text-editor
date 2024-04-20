use ropey::Rope;

pub type Cursor = usize;

pub fn cursor_add(cursor: Cursor, value: i32) -> Cursor {
    return (cursor as i32 + value).max(0) as Cursor;
}

pub struct Buffer {
    pub text: Rope,
    pub cursor: Cursor,
}

impl Buffer {
    pub fn find_line_position(&self, cursor: Cursor) -> usize {
        // find the char index of the cursor within the current line
        let line = self.text.byte_to_line(cursor);
        let line_start = self.text.line_to_char(line);
        cursor - line_start
    }

    pub fn get_movement_x(&self, cursor: Cursor, x: i32) -> Cursor {
        // move the cursor in by x. positive x -> move right; negative -> move left.
        //      automatically moves across lines when the end of line is reache
        (cursor as i64 + x as i64).clamp(0, self.text.len_chars() as i64 - 1) as Cursor
    }

    pub fn move_x(&mut self, x: i32) {
        self.cursor = self.get_movement_x(self.cursor, x);
    }

    pub fn get_movement_y(&self, cursor: Cursor, y: i32) -> Cursor {
        let current_y = self.text.byte_to_line(cursor);
        let new_y =
            (current_y as i64 + y as i64).clamp(0, (self.text.len_lines() - 1) as i64) as Cursor;
        let current_x = self.find_line_position(cursor);

        let new_x = current_x.clamp(0, self.text.line(new_y).len_chars() - 1);
        let new_cursor = self.text.line_to_char(new_y);

        self.get_movement_x(new_cursor, new_x as i32)
    }

    pub fn move_y(&mut self, y: i32) {
        self.cursor = self.get_movement_y(self.cursor, y);
    }

    pub fn get_end_of_line_cursor(&self, cursor: Cursor) -> Cursor {
        let y = self.text.char_to_line(cursor);
        let line_start = self.text.line_to_byte(y);
        let line_length = self.text.line(y).len_chars();
        line_start + line_length - 1
    }

    pub fn insert_after_cursor(&mut self, c: char) {
        self.text.insert_char(self.cursor, c);
    }
}

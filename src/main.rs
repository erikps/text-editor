use std::collections::HashMap;

use notan::draw::*;
use notan::log::debug;
use notan::prelude::*;
use ropey::iter::Chars;

type Cursor = usize;

fn cursor_add(cursor: Cursor, value: i32) -> Cursor {
    return (cursor as i32 + value).max(0) as Cursor;
}

#[derive(Debug, Clone)]
enum Motion {
    Left,
    Right,
    Up,
    Down,
    ForwardWord,
    ForwardWordEnd,
    BackWord,
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
    fn get_target(self, state: &State) -> Cursor {
        match self {
            Motion::ForwardWord => {
                let chars = state.text.chars_at(state.cursor);
                let is_alphanumeric_start = state.text.char(state.cursor).is_alphanumeric();
                let offset = skip_while(chars, |_, character| {
                    // skip to the next non-alphanumeric character
                    is_alphanumeric_start == character.is_alphanumeric()
                });
                state.cursor + offset
            }
            Motion::ForwardWordEnd => {
                let chars = state.text.chars_at(state.cursor);
                let is_alphanumeric_start =
                    state.text.char(state.cursor.max(1) + 1).is_alphanumeric();
                let offset = skip_while(chars, |_, character| {
                    // skip to the next non-alphanumeric character
                    is_alphanumeric_start == character.is_alphanumeric()
                });
                cursor_add(state.cursor, offset as i32 - 1)
            }
            Motion::BackWord => {
                let chars = state.text.chars_at(state.cursor).reversed();
                let is_alphanumeric_start =
                    state.text.char(state.cursor.max(1) - 1).is_alphanumeric();
                let offset = skip_while(chars, |_, character| {
                    // skip to the next non-alphanumeric character
                    is_alphanumeric_start == character.is_alphanumeric()
                });
                cursor_add(state.cursor, -(offset as i32))
            }
            Motion::Left => state.get_movement_x(state.cursor, -1),
            Motion::Down => state.get_movement_y(state.cursor, 1),
            Motion::Up => state.get_movement_y(state.cursor, -1),
            Motion::Right => state.get_movement_x(state.cursor, 1),

            _ => state.cursor,
        }
    }
}

#[derive(PartialEq, Eq, Hash)]
struct Shortcut {
    key: KeyCode,
    ctrl: bool,
    alt: bool,
    shift: bool,
}

impl Shortcut {
    fn new(key: KeyCode) -> Self {
        Shortcut {
            key,
            ctrl: false,
            alt: false,
            shift: false,
        }
    }

    fn shift(&mut self) -> &mut Self {
        self.shift = true;
        self
    }

    fn ctrl(&mut self) -> &mut Self {
        self.ctrl = true;
        self
    }

    fn alt(&mut self) -> &mut Self {
        self.alt = true;
        self
    }
}

type KeyBindings<T> = HashMap<Shortcut, T>;
type ActionBindings = KeyBindings<Action>;
type MotionBindings = KeyBindings<Motion>;

#[derive(Debug, Clone)]
enum Action {
    Delete,
    Replace,
}

#[derive(Debug, PartialEq, Clone)]
enum Mode {
    Normal,
    Input,
}

#[derive(AppState)]
struct State {
    font: Font,
    line_height: f32,

    cursor: Cursor,
    text: ropey::Rope,

    mode: Mode,

    action: Option<Action>,

    action_bindings: ActionBindings,
    motion_bindings: MotionBindings,

    last_time: f32,
    initial_movement_delay: f32,
    inter_movement_delay: f32,
}

impl State {
    pub fn find_line_position(&self, cursor: Cursor) -> usize {
        // find the char index of the cursor within the current line
        let line = self.text.byte_to_line(cursor);
        let line_start = self.text.line_to_char(line);
        cursor - line_start
    }

    pub fn get_movement_x(&self, cursor: Cursor, x: i32) -> Cursor {
        // move the cursor in by x. positive x -> move right; negative -> move left.
        //      automatically moves across lines when the end of line is reached
        (cursor as i64 + x as i64).clamp(0, self.text.len_chars() as i64) as Cursor
    }

    pub fn move_x(&mut self, x: i32) {
        self.cursor = self.get_movement_x(self.cursor, x);
    }

    pub fn get_movement_y(&self, cursor: Cursor, y: i32) -> Cursor {
        let line = self.text.byte_to_line(cursor);
        let target_line = (line as i64 + y as i64).clamp(0, self.text.len_lines() as i64) as Cursor;
        let previous_line_position = self.find_line_position(cursor);
        let new_line_position = previous_line_position.clamp(0, self.text.line(line).len_chars());
        let new_position = self.text.line_to_char(target_line);
        self.get_movement_x(new_line_position, new_position as i32)
    }

    pub fn move_y(&mut self, y: i32) {
        self.cursor = self.get_movement_y(self.cursor, y);
    }
}

const INPUT_DELAY: f32 = 0.05;

#[notan_main]
fn main() -> Result<(), String> {
    notan::init_with(setup)
        .add_config(DrawConfig)
        .event(event)
        .update(update)
        .draw(draw)
        .build()
}

fn setup(gfx: &mut Graphics) -> State {
    let font = gfx
        .create_font(include_bytes!("assets/FiraCode-Regular.ttf"))
        .unwrap();

    let text_string = r#"print('Hello World')
print('!=')
print('!')
print('!')
print('!')
print('!')
print('!')
print('!')
print('!')"#;

    let mut action_bindings = ActionBindings::new();
    let mut motion_bindings = MotionBindings::new();

    action_bindings.insert(Shortcut::new(KeyCode::D), Action::Delete);
    action_bindings.insert(Shortcut::new(KeyCode::C), Action::Replace);

    motion_bindings.insert(Shortcut::new(KeyCode::H), Motion::Left);
    motion_bindings.insert(Shortcut::new(KeyCode::J), Motion::Down);
    motion_bindings.insert(Shortcut::new(KeyCode::K), Motion::Up);
    motion_bindings.insert(Shortcut::new(KeyCode::L), Motion::Right);

    motion_bindings.insert(Shortcut::new(KeyCode::W), Motion::ForwardWord);
    motion_bindings.insert(Shortcut::new(KeyCode::E), Motion::ForwardWordEnd);
    motion_bindings.insert(Shortcut::new(KeyCode::B), Motion::BackWord);

    State {
        font,
        line_height: 16.0,

        cursor: 0,
        text: ropey::Rope::from(text_string),

        mode: Mode::Normal,

        action: Option::None,
        action_bindings,
        motion_bindings,

        last_time: 0.0,
        inter_movement_delay: 0.05,
        initial_movement_delay: 0.005,
    }
}

fn event(state: &mut State, event: Event) {
    if state.mode == Mode::Input {
        match event {
            Event::ReceivedCharacter(c) if c != '\u{7f}' && !c.is_control() => {
                state.text.insert_char(state.cursor, c);
                state.move_x(1);
            }
            _ => {}
        }
    }
}

fn was_pressed_or_held(app: &mut App, state: &mut State, key_code: KeyCode) -> bool {
    let pressed = app.keyboard.was_pressed(key_code)
        || ((app.keyboard.down_delta(key_code) > state.initial_movement_delay)
            && app.timer.elapsed_f32() - state.last_time > state.inter_movement_delay);
    if pressed {
        state.last_time = app.timer.elapsed_f32();
    }
    pressed
}

fn get_action_input(app: &App, state: &mut State) -> Option<Action> {
    for (shortcut, action) in state.action_bindings.iter() {
        if app.keyboard.was_pressed(shortcut.key) {
            return Some(action.clone());
        }
    }
    Option::None
}

fn get_motion_input(app: &App, state: &mut State) -> Option<Motion> {
    let mut result: Option<Motion> = None;

    for (shortcut, motion) in state.motion_bindings.iter() {
        let pressed = app.keyboard.was_pressed(shortcut.key)
            || ((app.keyboard.down_delta(shortcut.key) > state.initial_movement_delay)
                && app.timer.elapsed_f32() - state.last_time > state.inter_movement_delay);
        if pressed {
            result = Some(motion.clone());
            state.last_time = app.timer.elapsed_f32();
        }

    }
    result
}

fn update(app: &mut App, state: &mut State) {
    if app.keyboard.was_pressed(KeyCode::Return) && app.keyboard.alt() {
        let is_fullscreen = app.window().is_fullscreen();
        app.window().set_fullscreen(!is_fullscreen);
    }

    // if there is a new action input, replace the previous
    let input_action = get_action_input(app, state);
    if let Some(new_action) = input_action {
        state.action = Some(new_action.clone());
    }

    match state.mode {
        Mode::Normal => {
            let action = state.action.clone();

            if let Some(motion) = get_motion_input(app, state) {
                let target = motion.get_target(state);
                if let Some(action) = action {
                    match action {
                        Action::Delete => {
                            if state.cursor <= target {
                                state.text.remove(state.cursor..target);
                            } else {
                                state.text.remove(target..state.cursor);
                                state.cursor = target;
                            }
                        }
                        Action::Replace => {
                            state.mode = Mode::Input;
                            if state.cursor <= target {
                                state.text.remove(state.cursor..target);
                            } else {
                                state.text.remove(target..state.cursor);
                                state.cursor = target;
                            }
                        }
                    }
                    state.action = None;
                } else {
                    state.cursor = target;
                }
            }

            if app.keyboard.is_down(KeyCode::I) {
                state.mode = Mode::Input;
                return;
            }

            if was_pressed_or_held(app, state, KeyCode::Equals) && app.keyboard.ctrl() {
                state.line_height += 1f32;
            }

            if was_pressed_or_held(app, state, KeyCode::Minus) && app.keyboard.ctrl() {
                state.line_height = (state.line_height - 1f32).max(1f32);
            }

            if app.keyboard.was_pressed(KeyCode::A) {
                state.move_x(1);
                state.mode = Mode::Input;
                return;
            }

            if app.keyboard.was_pressed(KeyCode::X) {
                state.text.remove(state.cursor..state.cursor + 1);
                state.move_x(0);
            }

            // if was_pressed_or_held(app, state, KeyCode::J) {
            //     state.move_y(1);
            // }

            // if was_pressed_or_held(app, state, KeyCode::K) {
            //     state.move_y(-1);
            // }

            // if was_pressed_or_held(app, state, KeyCode::L) {
            //     state.move_x(1);
            // }

            // if was_pressed_or_held(app, state, KeyCode::H) {
            //     state.move_x(-1);
            // }
        }
        Mode::Input => {
            if app.keyboard.was_pressed(KeyCode::Escape) {
                state.mode = Mode::Normal;
                return;
            }

            if was_pressed_or_held(app, state, KeyCode::Back) {
                if state.cursor > 0 {
                    state.text.remove(state.cursor - 1..state.cursor);
                    state.move_x(-1);
                }
            }

            if was_pressed_or_held(app, state, KeyCode::Return) {
                state.text.insert_char(state.cursor, '\n');
                state.move_x(1)
            }
        }
    }
}

fn draw(gfx: &mut Graphics, state: &mut State) {
    let mut draw = gfx.create_draw();
    draw.clear(Color::BLACK);

    draw.text(&state.font, "0").color(Color::TRANSPARENT);
    let bounds = draw.last_text_bounds();
    let char_width = bounds.width;

    let cursor_line = state.text.char_to_line(state.cursor);
    let cursor_line_position = state.find_line_position(state.cursor);

    for (index, line) in state.text.lines().enumerate() {
        let y_position = index as f32 * state.line_height;

        draw.text(&state.font, &line.to_string())
            .position(0.0, y_position)
            .size(state.line_height);

        if cursor_line == index {
            let x_position = char_width * cursor_line_position as f32;

            match state.mode {
                Mode::Normal => {
                    draw.rect((x_position, y_position), (char_width, state.line_height));
                }
                Mode::Input => {
                    draw.line(
                        (x_position, y_position),
                        (x_position, y_position + state.line_height),
                    );
                }
            }
        }
    }
    gfx.render(&draw);
}

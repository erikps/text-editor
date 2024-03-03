use std::collections::HashMap;

use notan::draw::*;
use notan::log::debug;
use notan::prelude::*;
use ropey::iter::Chars;

type Cursor = usize;

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
            Motion::BackWord => {
                let chars = state.text.chars_at(state.cursor).reversed();
                let is_alphanumeric_start = state.text.char(state.cursor).is_alphanumeric();
                let offset = skip_while(chars, |_, character| {
                    // skip to the next non-alphanumeric character
                    is_alphanumeric_start == character.is_alphanumeric()
                });
                state.cursor - offset - 1
            }
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
    pub fn find_cursor_line_position(&self) -> usize {
        // find the char index of the cursor within the current line
        let line = self.text.byte_to_line(self.cursor);
        let line_start = self.text.line_to_char(line);
        self.cursor - line_start
    }

    pub fn move_x(&mut self, x: i32) {
        // move the cursor in by x. positive x -> move right; negative -> move left.
        //      automatically moves across lines when the end of line is reached
        self.cursor =
            (self.cursor as i64 + x as i64).clamp(0, self.text.len_chars() as i64) as Cursor;
    }

    pub fn move_y(&mut self, y: i32) {
        let line = self.text.byte_to_line(self.cursor);
        let target_line = (line as i64 + y as i64).clamp(0, self.text.len_lines() as i64) as Cursor;
        let previous_line_position = self.find_cursor_line_position();
        let new_line_position = previous_line_position.clamp(0, self.text.line(line).len_chars());
        self.cursor = self.text.line_to_char(target_line);
        self.move_x(new_line_position as i32);
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
    for (shortcut, motion) in state.motion_bindings.iter() {
        if app.keyboard.was_pressed(shortcut.key) {
            return Some(motion.clone());
        }
    }
    Option::None
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
                        _ => {}
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

            if was_pressed_or_held(app, state, KeyCode::J) {
                state.move_y(1);
            }

            if was_pressed_or_held(app, state, KeyCode::K) {
                state.move_y(-1);
            }

            if was_pressed_or_held(app, state, KeyCode::L) {
                state.move_x(1);
            }

            if was_pressed_or_held(app, state, KeyCode::H) {
                state.move_x(-1);
            }
        }
        Mode::Input => {
            if app.keyboard.was_pressed(KeyCode::Escape) {
                state.mode = Mode::Normal;
                return;
            }

            if app.keyboard.was_pressed(KeyCode::Back) {
                if state.cursor > 0 {
                    state.text.remove(state.cursor - 1..state.cursor);
                    state.move_x(-1);
                }
            }

            // if app.keyboard.was_pressed(KeyCode::Return) {
            //     let (left_string, right_string) =
            //         state.text[state.cursor.y].split_at(state.cursor.x);
            //     let left = String::from(left_string);
            //     let right = String::from(right_string);
            //     state.text[state.cursor.y] = left;
            //     state.text.insert(state.cursor.y + 1, right);
            //     state.cursor.y += 1;
            //     state.cursor.x = 0;
            // }
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
    let cursor_line_position = state.find_cursor_line_position();

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

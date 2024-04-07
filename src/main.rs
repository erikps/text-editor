use std::collections::HashMap;

use notan::app::Plugins;
use notan::draw::*;
use notan::prelude::*;
use notan_egui::{EguiConfig, EguiPluginSugar};
use ropey::iter::Chars;

type Cursor = usize;

fn cursor_add(cursor: Cursor, value: i32) -> Cursor {
    return (cursor as i32 + value).max(0) as Cursor;
}

struct Buffer {
    text: ropey::Rope,
    cursor: Cursor,
}

struct Viewport {
    buffer: Buffer,
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
    fn get_target(self, state: &State) -> Cursor {
        match self {
            Motion::ForwardWord => {
                let chars = state.text.chars_at(state.cursor);
                let is_alphanumeric_start = state.text.char(state.cursor).is_alphanumeric();
                let offset = skip_while(chars, |_, character| {
                    // skip to the next non-alphanumeric character
                    is_alphanumeric_start == character.is_alphanumeric()
                });
                state.get_movement_x(state.cursor, offset as i32)
            }
            Motion::ForwardWordEnd => {
                let chars = state.text.chars_at(state.get_movement_x(state.cursor, 1));
                let is_alphanumeric_start = state
                    .text
                    .char((state.cursor.max(1) + 1).min(state.text.len_chars() - 1))
                    .is_alphanumeric();

                let offset = skip_while(chars, |_, character| {
                    // skip to the next non-alphanumeric character
                    is_alphanumeric_start == character.is_alphanumeric()
                }) + 1;
                state.get_movement_x(state.cursor, offset as i32 - 1)
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

            Motion::EndOfLine => state.get_end_of_line_cursor(state.cursor),

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

    fn shift(mut self) -> Self {
        self.shift = true;
        self
    }

    fn ctrl(mut self) -> Self {
        self.ctrl = true;
        self
    }

    fn alt(&mut self) -> &mut Self {
        self.alt = true;
        self
    }
}

#[derive(PartialEq, Eq, Hash, Clone)]
enum ModeChange {
    Insert,
    InsertAfter,
    InsertEnd,
    InsertStart,
    Escape,
}

type KeyBindings<T> = HashMap<Shortcut, T>;
type ActionBindings = KeyBindings<Action>;
type MotionBindings = KeyBindings<Motion>;
type ModeChangeBindings = KeyBindings<ModeChange>;

struct Keymap {
    action_bindings: ActionBindings,
    motion_bindings: MotionBindings,
    mode_change_bindings: HashMap<Mode, ModeChangeBindings>,
}

#[derive(Debug, Clone)]
enum Action {
    Delete,
    Replace,
}

#[derive(Debug, PartialEq, Clone, Hash, Eq)]
enum Mode {
    Normal,
    Insert,
}

#[derive(AppState)]
struct State {
    font: Font,
    line_height: f32,

    cursor: Cursor,
    text: ropey::Rope,

    mode: Mode,

    action: Option<Action>,

    keymap: Keymap,

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
}

const INPUT_DELAY: f32 = 0.05;

#[notan_main]
fn main() -> Result<(), String> {
    let win = WindowConfig::new()
        .set_min_size(0, 0)
        .set_size(800, 600)
        .set_position(0, 0)
        .set_resizable(true)
        .set_vsync(true)
        .set_lazy_loop(true)
        .set_high_dpi(true);

    notan::init_with(setup)
        .add_config(DrawConfig)
        .add_config(EguiConfig)
        .add_config(win)
        .event(event)
        .update(update)
        .draw(draw)
        .build()
}

fn setup(app: &mut App, gfx: &mut Graphics, plugins: &mut Plugins) -> State {
    plugins.egui(|ctx| {
        ctx.set_pixels_per_point(app.window().dpi() as f32);
    });

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
    let mut mode_change_bindings: HashMap<Mode, ModeChangeBindings> = HashMap::new();
    let mut insert_mode_change_bindings = ModeChangeBindings::new();
    let mut normal_mode_change_bindings = ModeChangeBindings::new();

    action_bindings.insert(Shortcut::new(KeyCode::D), Action::Delete);
    action_bindings.insert(Shortcut::new(KeyCode::C), Action::Replace);

    motion_bindings.insert(Shortcut::new(KeyCode::H), Motion::Left);
    motion_bindings.insert(Shortcut::new(KeyCode::J), Motion::Down);
    motion_bindings.insert(Shortcut::new(KeyCode::K), Motion::Up);
    motion_bindings.insert(Shortcut::new(KeyCode::L), Motion::Right);

    motion_bindings.insert(Shortcut::new(KeyCode::W), Motion::ForwardWord);
    motion_bindings.insert(Shortcut::new(KeyCode::E), Motion::ForwardWordEnd);
    motion_bindings.insert(Shortcut::new(KeyCode::B), Motion::BackWord);
    motion_bindings.insert(Shortcut::new(KeyCode::Key4).shift(), Motion::EndOfLine);

    normal_mode_change_bindings.insert(Shortcut::new(KeyCode::I), ModeChange::Insert);
    normal_mode_change_bindings.insert(Shortcut::new(KeyCode::A).shift(), ModeChange::InsertEnd);
    normal_mode_change_bindings.insert(Shortcut::new(KeyCode::A), ModeChange::InsertAfter);
    normal_mode_change_bindings.insert(Shortcut::new(KeyCode::I).shift(), ModeChange::InsertStart);

    insert_mode_change_bindings.insert(Shortcut::new(KeyCode::Escape), ModeChange::Escape);
    insert_mode_change_bindings.insert(Shortcut::new(KeyCode::LBracket).ctrl(), ModeChange::Escape);

    mode_change_bindings.insert(Mode::Normal, normal_mode_change_bindings);
    mode_change_bindings.insert(Mode::Insert, insert_mode_change_bindings);

    let keymap = Keymap {
        motion_bindings,
        action_bindings,
        mode_change_bindings,
    };

    State {
        font,
        line_height: 16.0,

        cursor: 0,
        text: ropey::Rope::from(text_string),

        mode: Mode::Normal,

        action: Option::None,
        keymap,

        last_time: 0.0,
        inter_movement_delay: 0.05,
        initial_movement_delay: 0.005,
    }
}

fn event(state: &mut State, event: Event) {
    if state.mode == Mode::Insert {
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

fn get_action_input(app: &App, state: &Keymap) -> Option<Action> {
    for (shortcut, action) in state.action_bindings.iter() {
        if app.keyboard.was_pressed(shortcut.key) {
            return Some(action.clone());
        }
    }
    Option::None
}

fn get_motion_input(app: &App, state: &mut State) -> Option<Motion> {
    let mut result: Option<Motion> = None;

    for (shortcut, motion) in state.keymap.motion_bindings.iter() {
        let shift = shortcut.shift == app.keyboard.shift();
        let control = shortcut.ctrl == app.keyboard.ctrl();
        let alt = shortcut.alt == app.keyboard.alt();
        let modifiers_satisfied = shift && control && alt;

        let just_pressed = app.keyboard.was_pressed(shortcut.key);
        let continuous_pressed = (app.keyboard.down_delta(shortcut.key)
            > state.initial_movement_delay)
            && app.timer.elapsed_f32() - state.last_time > state.inter_movement_delay;
        let pressed = (just_pressed || continuous_pressed) && modifiers_satisfied;
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

    if state.mode == Mode::Normal {
        // if there is a new action input, replace the previous
        let input_action = get_action_input(app, &state.keymap);
        if let Some(new_action) = input_action {
            state.action = Some(new_action.clone());
            println!("{:?}", new_action);
        }
    }

    let mut enacted_mode_change: Option<ModeChange> = None;
    for mode in state.keymap.mode_change_bindings.keys().cloned() {
        for (shortcut, mode_change) in state.keymap.mode_change_bindings.get(&mode).unwrap() {
            let shift = shortcut.shift == app.keyboard.shift();
            let control = shortcut.ctrl == app.keyboard.ctrl();
            let alt = shortcut.alt == app.keyboard.alt();
            let modifiers_satisfied = shift && control && alt;

            if app.keyboard.was_pressed(shortcut.key) && modifiers_satisfied {
                enacted_mode_change = Some((mode_change).clone());
            }
        }
    }

    if let Some(mode_change) = enacted_mode_change {
        match mode_change {
            ModeChange::Insert => {
                state.mode = Mode::Insert;
            }
            ModeChange::InsertAfter => {
                state.mode = Mode::Insert;
                state.move_x(1);
            }
            ModeChange::InsertEnd => {
                state.mode = Mode::Insert;
            }
            ModeChange::InsertStart => {
                state.mode = Mode::Insert;
            }
            ModeChange::Escape => {
                state.mode = Mode::Normal;
            }
        }
        return;
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
                            state.mode = Mode::Insert;
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

            // if app.keyboard.is_down(KeyCode::I) {
            //     state.mode = Mode::Insert;
            //     return;
            // }

            if was_pressed_or_held(app, state, KeyCode::Equals) && app.keyboard.ctrl() {
                state.line_height += 1f32;
            }

            if was_pressed_or_held(app, state, KeyCode::Minus) && app.keyboard.ctrl() {
                state.line_height = (state.line_height - 1f32).max(1f32);
            }

            if app.keyboard.was_pressed(KeyCode::A) {
                state.move_x(1);
                state.mode = Mode::Insert;
                return;
            }

            if app.keyboard.was_pressed(KeyCode::X) {
                state.text.remove(state.cursor..state.cursor + 1);
                state.move_x(0);
            }
        }
        Mode::Insert => {
            // if app.keyboard.was_pressed(KeyCode::Escape) {
            //     state.mode = Mode::Normal;
            //     return;
            // }

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
                Mode::Insert => {
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

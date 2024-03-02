use std::collections::HashMap;

use notan::draw::*;
use notan::prelude::*;

#[derive(Debug)]
struct Cursor {
    x: usize,
    y: usize,
}

impl Cursor {
    fn new(x: usize, y: usize) -> Self {
        Cursor { x, y }
    }
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

impl Motion {
    fn get_movement(self: &Motion, app: &App, state: &State) -> Cursor {
        match self {
            Motion::ForwardWord => {
                let line = state.current_line();
                
                Cursor::new(state.cursor.x, state.cursor.y)
            },
            _ => Cursor::new(state.cursor.x, state.cursor.y),
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
    lines: Vec<String>,

    mode: Mode,

    action: Option<Action>,

    action_bindings: ActionBindings,
    motion_bindings: MotionBindings,

    last_time: f32,
    initial_movement_delay: f32,
    inter_movement_delay: f32,
}

impl State {
    pub fn move_x(&mut self, x: i32) {
        let max_value = self.lines[self.cursor.y].len() as i32 - 1;
        let new_x = (self.cursor.x as i32 + x).clamp(0, max_value);
        self.cursor.x = new_x as usize;
    }

    pub fn move_y(&mut self, y: i32) {
        let max_value = self.lines.len() as i32 - 1;
        let new_y = (self.cursor.y as i32 + y).clamp(0, max_value);
        self.cursor.y = new_y as usize;
        self.move_x(0);
    }

    pub fn current_line(&self) -> String {
        self.lines[self.cursor.y].clone()
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

    let lines = vec![
        String::from("print('Hello World')"),
        String::from("print('!=')"),
        String::from("print('!')"),
        String::from("print('!')"),
        String::from("print('!')"),
        String::from("print('!')"),
        String::from("print('!')"),
        String::from("print('!')"),
        String::from("print('!')"),
    ];

    let mut action_bindings = ActionBindings::new();
    let mut motion_bindings = MotionBindings::new();

    action_bindings.insert(Shortcut::new(KeyCode::D), Action::Delete);
    motion_bindings.insert(Shortcut::new(KeyCode::W), Motion::ForwardWord);

    State {
        font,
        line_height: 16.0,

        cursor: Cursor { x: 0, y: 0 },
        lines,

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
            Event::ReceivedCharacter(c) if c != '\u{7f}' && !c.is_control()  => { //&& c.is_ascii()
                state.lines[state.cursor.y].insert(state.cursor.x, c);
                state.cursor.x += 1;
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

fn get_motion_from_key_press(app: &App, state: &mut State) -> Option<Motion> {
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

    match state.mode {
        Mode::Normal => {
            let action = state.action.clone();

            if let Some(action) = action {
                match action {
                    Action::Delete => {}
                    _ => {}
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
                state.lines[state.cursor.y].remove(state.cursor.x);
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
                if state.cursor.x > 0 {
                    state.lines[state.cursor.y].remove(state.cursor.x - 1);
                    state.cursor.x = state.cursor.x - 1
                }
            }

            if app.keyboard.was_pressed(KeyCode::Return) {
                let (left_string, right_string) =
                    state.lines[state.cursor.y].split_at(state.cursor.x);
                let left = String::from(left_string);
                let right = String::from(right_string);
                state.lines[state.cursor.y] = left;
                state.lines.insert(state.cursor.y + 1, right);
                state.cursor.y += 1;
                state.cursor.x = 0;
            }
        }
    }
}

fn draw(gfx: &mut Graphics, state: &mut State) {
    let mut draw = gfx.create_draw();
    draw.clear(Color::BLACK);
    

    for (index, line) in state.lines.iter().enumerate() {
        let y_position = index as f32 * state.line_height;

        draw.text(&state.font, &line)
            .position(0.0, y_position)
            .size(state.line_height);
        let last_bounds = draw.last_text_bounds();

        if state.cursor.y == index {
            
            let char_width = last_bounds.max_x() / line.len() as f32;
            let x_position = char_width * state.cursor.x as f32;

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

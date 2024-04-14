mod action;
mod buffer;
mod io;
mod motion;
mod state;

use action::*;
use buffer::Buffer;
use io::{load, save};
use motion::*;
use notan_egui::TextBuffer;
use state::*;

use std::collections::HashMap;

use notan::app::Plugins;
use notan::draw::*;
use notan::prelude::*;
use notan_egui::{EguiConfig, EguiPluginSugar};

const TAB_SIZE: usize = 4;
const COMMAND_BOX_HEIGHT: f32 = 20.0;
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
    let mut command_mode_change_bindings = ModeChangeBindings::new();

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
    normal_mode_change_bindings.insert(
        Shortcut::new(KeyCode::Semicolon).shift(),
        ModeChange::EnterCommand,
    );

    insert_mode_change_bindings.insert(Shortcut::new(KeyCode::Escape), ModeChange::Escape);
    insert_mode_change_bindings.insert(Shortcut::new(KeyCode::LBracket).ctrl(), ModeChange::Escape);

    command_mode_change_bindings.insert(Shortcut::new(KeyCode::Escape), ModeChange::Escape);
    command_mode_change_bindings
        .insert(Shortcut::new(KeyCode::LBracket).ctrl(), ModeChange::Escape);

    mode_change_bindings.insert(Mode::Normal, normal_mode_change_bindings);
    mode_change_bindings.insert(Mode::Insert, insert_mode_change_bindings);
    mode_change_bindings.insert(Mode::Command, command_mode_change_bindings);

    let keymap = Keymap {
        motion_bindings,
        action_bindings,
        mode_change_bindings,
    };

    State {
        font,
        line_height: 16.0,

        buffer: Buffer {
            cursor: 0,
            text: ropey::Rope::from(text_string),
        },
        command_line: String::new(),

        mode: Mode::Normal,

        action: Option::None,
        keymap,

        last_time: 0.0,
        inter_movement_delay: 0.05,
        initial_movement_delay: 0.005,
    }
}

fn event(state: &mut State, event: Event) {
    match state.mode {
        Mode::Normal => {}
        Mode::Insert => match event {
            Event::ReceivedCharacter(c) if c != '\u{7f}' && !c.is_control() => {
                state.buffer.text.insert_char(state.buffer.cursor, c);
                state.buffer.move_x(1);
            }
            _ => {}
        },
        Mode::Command => match event {
            Event::ReceivedCharacter(c) if c != '\u{7f}' && !c.is_control() => {
                state.command_line.push(c);
                println!("{}", &state.command_line);
            }
            _ => {}
        },
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

fn execute_command(state: &mut State) {
    println!("{}", state.command_line);

    match state.command_line.clone() {
        x if x.get(1..2) == Some("w") => {
            let mut splits = x.split(" ");
            splits.next();
            if let Some(string) = splits.next() {
                let result = save(&state.buffer.text, string);
                println!("{:#}", result.is_ok());
            }

        }
        _ => {}
    }

    state.command_line.clear();
    state.mode = Mode::Normal;
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

            if mode == state.mode && app.keyboard.was_pressed(shortcut.key) && modifiers_satisfied {
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
                state.buffer.move_x(1);
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
            ModeChange::EnterCommand => {
                state.mode = Mode::Command;
                state.command_line.clear();
                state.command_line.push(':');
            }
        }
        return;
    }
    match state.mode {
        Mode::Normal => {
            let action = state.action.clone();

            if let Some(motion) = get_motion_input(app, state) {
                let target = motion.get_target(&state.buffer);
                if let Some(action) = action {
                    match action {
                        Action::Delete => {
                            if state.buffer.cursor <= target {
                                state.buffer.text.remove(state.buffer.cursor..target);
                            } else {
                                state.buffer.text.remove(target..state.buffer.cursor);
                                state.buffer.cursor = target;
                            }
                        }
                        Action::Replace => {
                            state.mode = Mode::Insert;
                            if state.buffer.cursor <= target {
                                state.buffer.text.remove(state.buffer.cursor..target);
                            } else {
                                state.buffer.text.remove(target..state.buffer.cursor);
                                state.buffer.cursor = target;
                            }
                        }
                    }
                    state.action = None;
                } else {
                    state.buffer.cursor = target;
                }
            }

            if was_pressed_or_held(app, state, KeyCode::Equals) && app.keyboard.ctrl() {
                state.line_height += 1f32;
            }

            if was_pressed_or_held(app, state, KeyCode::Minus) && app.keyboard.ctrl() {
                state.line_height = (state.line_height - 1f32).max(1f32);
            }

            if app.keyboard.was_pressed(KeyCode::A) {
                state.buffer.move_x(1);
                state.mode = Mode::Insert;
                return;
            }

            if app.keyboard.was_pressed(KeyCode::X) {
                state
                    .buffer
                    .text
                    .remove(state.buffer.cursor..state.buffer.cursor + 1);
                state.buffer.move_x(0);
            }
        }
        Mode::Insert => {
            if was_pressed_or_held(app, state, KeyCode::Back) {
                if state.buffer.cursor > 0 {
                    state
                        .buffer
                        .text
                        .remove(state.buffer.cursor - 1..state.buffer.cursor);
                    state.buffer.move_x(-1);
                }
            }

            if was_pressed_or_held(app, state, KeyCode::Return) {
                state.buffer.text.insert_char(state.buffer.cursor, '\n');
                state.buffer.move_x(1)
            }

            if was_pressed_or_held(app, state, KeyCode::Tab) {
                state
                    .buffer
                    .text
                    .insert(state.buffer.cursor, &" ".repeat(TAB_SIZE));
                state.buffer.move_x(TAB_SIZE as i32);
            }

            if was_pressed_or_held(app, state, KeyCode::Delete) {
                let length = state.buffer.text.len_chars();
                state
                    .buffer
                    .text
                    .remove(state.buffer.cursor..(state.buffer.cursor + 1).min(length));
            }
        }

        Mode::Command => {
            if was_pressed_or_held(app, state, KeyCode::Return) {
                execute_command(state);
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

    let cursor_line = state.buffer.text.char_to_line(state.buffer.cursor);
    let cursor_line_position = state.buffer.find_line_position(state.buffer.cursor);

    for (index, line) in state.buffer.text.lines().enumerate() {
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
                Mode::Command => {}
            }
        }
    }

    // render command line at the bottom of the screen
    if state.mode == Mode::Command {
        let (w, h) = gfx.size();
        draw.rect((0.0, h as f32 - COMMAND_BOX_HEIGHT), (w as f32, h as f32));
        draw.text(&state.font, &state.command_line)
            .position(0.0, h as f32 - COMMAND_BOX_HEIGHT)
            .color(Color::BLACK);
    }
    gfx.render(&draw);
}

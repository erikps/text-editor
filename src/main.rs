mod action;
mod buffer;
mod commands;
mod highlight;
mod io;
mod motion;
mod state;

use commands::{get_standard_commands, prepare_command};
use highlight::convert_color;
use highlight::highlight;

use action::*;
use buffer::Buffer;
use io::{load, save};
use motion::*;
use state::*;

use std::collections::HashMap;

use notan::app::Plugins;
use notan::draw::*;
use notan::prelude::*;
use notan_egui::{EguiConfig, EguiPluginSugar};

const TAB_SIZE: usize = 4;
const COMMAND_BOX_PADDING: f32 = 8.0;
const SHOW_LINE_NUMBERS: bool = true;

#[notan_main]
fn main() -> Result<(), String> {
    let win = WindowConfig::new()
        .set_min_size(0, 0)
        .set_size(800, 400)
        .set_position(0, 0)
        .set_resizable(true)
        .set_vsync(true)
        .set_lazy_loop(true)
        .set_high_dpi(true)
        .set_title("Text Editor");

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

    let text_string = r#"def fib(number):
    if number == 0:
        return 0
    prev, current = 0, 1
    for _ in range(number - 1):
        temp = current
        current += prev
        prev = temp
    return current

print(fib(0))"#;

    let mut action_bindings = ActionBindings::new();
    let mut motion_bindings = MotionBindings::new();
    let mut mode_change_bindings: HashMap<Mode, ModeChangeBindings> = HashMap::new();
    let mut insert_mode_change_bindings = ModeChangeBindings::new();
    let mut normal_mode_change_bindings = ModeChangeBindings::new();
    let mut command_mode_change_bindings = ModeChangeBindings::new();
    let mut quick_menu_mode_change_bindings = ModeChangeBindings::new();

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
    normal_mode_change_bindings.insert(Shortcut::new(KeyCode::Space), ModeChange::EnterQuickMenu);

    insert_mode_change_bindings.insert(Shortcut::new(KeyCode::Escape), ModeChange::Escape);
    insert_mode_change_bindings.insert(Shortcut::new(KeyCode::LBracket).ctrl(), ModeChange::Escape);

    command_mode_change_bindings.insert(Shortcut::new(KeyCode::Escape), ModeChange::Escape);
    command_mode_change_bindings
        .insert(Shortcut::new(KeyCode::LBracket).ctrl(), ModeChange::Escape);

    quick_menu_mode_change_bindings.insert(Shortcut::new(KeyCode::Escape), ModeChange::Escape);

    mode_change_bindings.insert(Mode::Normal, normal_mode_change_bindings);
    mode_change_bindings.insert(Mode::Insert, insert_mode_change_bindings);
    mode_change_bindings.insert(Mode::Command, command_mode_change_bindings);
    mode_change_bindings.insert(Mode::QuickMenu, quick_menu_mode_change_bindings);

    let commands = get_standard_commands();

    let keymap = Keymap {
        motion_bindings,
        action_bindings,
        mode_change_bindings,
    };

    let buffers = vec![
        Buffer::new(ropey::Rope::from(text_string), None),
        Buffer::new(
            ropey::Rope::from(String::from("print('Hello, it\\'s me!')")),
            None,
        ),
    ];

    let editor = Editor {
        buffers,
        current_buffer_index: 0,
        command_line: String::new(),
        quick_menu_line: String::new(),
        mode: Mode::Normal,
        action: Option::None,
    };

    State {
        font,
        line_height: 16.0,

        editor,
        keymap,
        commands,

        last_time: 0.0,
        inter_movement_delay: 0.05,
        initial_movement_delay: 0.005,
    }
}

fn event(state: &mut State, event: Event) {
    match state.editor.mode.clone() {
        Mode::Normal => {}
        Mode::Insert => match event {
            Event::ReceivedCharacter(c) if c != '\u{7f}' && !c.is_control() => {
                state.editor.buffer().insert_after_cursor(c);
                state.editor.buffer().move_x(1);
            }
            _ => {}
        },
        Mode::Command => match event {
            Event::ReceivedCharacter(c) if c != '\u{7f}' && !c.is_control() => {
                state.editor.command_line.push(c);
            }
            _ => {}
        },
        Mode::QuickMenu => match event {
            Event::ReceivedCharacter(c) if c != '\u{7f}' && !c.is_control() => {
                state.editor.quick_menu_line.push(c);
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

fn get_action_input(app: &App, keymap: &Keymap) -> Option<Action> {
    for (shortcut, action) in keymap.action_bindings.iter() {
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

    if state.editor.mode == Mode::Normal {
        // if there is a new action input, replace the previous
        let input_action = get_action_input(app, &state.keymap);
        if let Some(new_action) = input_action {
            state.editor.action = Some(new_action.clone());
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

            if mode == state.editor.mode
                && app.keyboard.was_pressed(shortcut.key)
                && modifiers_satisfied
            {
                enacted_mode_change = Some((mode_change).clone());
            }
        }
    }

    if let Some(mode_change) = enacted_mode_change {
        match mode_change {
            ModeChange::Insert => {
                state.editor.mode = Mode::Insert;
            }
            ModeChange::InsertAfter => {
                state.editor.mode = Mode::Insert;
                state.editor.buffer().move_x(1);
            }
            ModeChange::InsertEnd => {
                state.editor.mode = Mode::Insert;
            }
            ModeChange::InsertStart => {
                state.editor.mode = Mode::Insert;
            }
            ModeChange::Escape => {
                state.editor.mode = Mode::Normal;
            }
            ModeChange::EnterCommand => {
                state.editor.mode = Mode::Command;
                state.editor.command_line.clear();
                state.editor.command_line.push(':');
            }
            ModeChange::EnterQuickMenu => {
                state.editor.mode = Mode::QuickMenu;
                state.editor.quick_menu_line.clear();
            }
        }
        return;
    }

    match state.editor.mode {
        Mode::Normal => {
            let action = state.editor.action.clone();

            if let Some(motion) = get_motion_input(app, state) {
                let target = motion.get_target(&state.editor.buffer());
                if let Some(action) = action {
                    match action {
                        Action::Delete => {
                            let reached_target = state.editor.buffer().cursor <= target;
                            let cursor = state.editor.buffer().cursor;
                            if reached_target {
                                state.editor.buffer().text.remove(cursor..target);
                            } else {
                                state.editor.buffer().text.remove(target..cursor);
                                state.editor.buffer().cursor = target;
                            }
                        }
                        Action::Replace => {
                            state.editor.mode = Mode::Insert;
                            let cursor = state.editor.buffer().cursor;
                            if cursor <= target {
                                state.editor.buffer().text.remove(cursor..target);
                            } else {
                                state.editor.buffer().text.remove(target..cursor);
                                state.editor.buffer().cursor = target;
                            }
                        }
                    }
                    state.editor.action = None;
                } else {
                    state.editor.buffer().cursor = target;
                }
            }

            if was_pressed_or_held(app, state, KeyCode::Equals) && app.keyboard.ctrl() {
                state.line_height += 1f32;
            }

            if was_pressed_or_held(app, state, KeyCode::Minus) && app.keyboard.ctrl() {
                state.line_height = (state.line_height - 1f32).max(1f32);
            }

            if app.keyboard.was_pressed(KeyCode::A) {
                state.editor.buffer().move_x(1);
                state.editor.mode = Mode::Insert;
                return;
            }

            if app.keyboard.was_pressed(KeyCode::X) {
                let cursor = state.editor.buffer().cursor;
                state.editor.buffer().text.remove(cursor..cursor + 1);
                state.editor.buffer().move_x(0);
            }
        }
        Mode::Insert => {
            let cursor = state.editor.buffer().cursor;
            if was_pressed_or_held(app, state, KeyCode::Back) {
                if cursor > 0 {
                    state.editor.buffer().text.remove(cursor - 1..cursor);
                    state.editor.buffer().move_x(-1);
                }
            }

            if was_pressed_or_held(app, state, KeyCode::Return) {
                state.editor.buffer().insert_after_cursor('\n');
                state.editor.buffer().move_x(1)
            }

            if was_pressed_or_held(app, state, KeyCode::Tab) {
                let cursor = state.editor.buffer().cursor;
                state
                    .editor
                    .buffer()
                    .text
                    .insert(cursor, &" ".repeat(TAB_SIZE));
                state.editor.buffer().move_x(TAB_SIZE as i32);
            }

            if was_pressed_or_held(app, state, KeyCode::Delete) {
                let length = state.editor.buffer().text.len_chars();
                let cursor = state.editor.buffer().cursor;
                state
                    .editor
                    .buffer()
                    .text
                    .remove(cursor..(cursor + 1).min(length));
            }
        }

        Mode::Command => {
            if was_pressed_or_held(app, state, KeyCode::Return) {
                let result = prepare_command(&state.commands, &state.editor.command_line);
                match result {
                    Ok((parameters, command_index)) => {
                        (state.commands[command_index].execute)(parameters, &mut state.editor);
                    }
                    Err(error_message) => {
                        println!("{}", error_message);
                    }
                }
                state.editor.command_line.clear();
                state.editor.mode = Mode::Normal;
            }

            if was_pressed_or_held(app, state, KeyCode::Back) {
                state.editor.command_line.pop();
                if state.editor.command_line.is_empty() {
                    state.editor.mode = Mode::Normal;
                }
            }
        }

        Mode::QuickMenu => {
            if was_pressed_or_held(app, state, KeyCode::Back) {
                state.editor.quick_menu_line.pop();
            }
        }
    }
}

fn calculate_camera_offset(
    cursor_x: usize,
    cursor_y: usize,
    char_width: f32,
    char_height: f32,
    screen_size: (u32, u32),
) -> (f32, f32) {
    let margin_x = 8;
    let margin_y = 4;

    let (cursor_x, cursor_y) = (
        (cursor_x + margin_x + 1) as f32 * char_width,
        (cursor_y + margin_y + 1) as f32 * char_height,
    );

    let (screen_x, screen_y) = screen_size;
    (
        -(cursor_x - screen_x as f32).max(0.0),
        -(cursor_y - screen_y as f32).max(0.0),
    )
}

fn draw(gfx: &mut Graphics, state: &mut State) {
    let (theme, highlighted_lines) =
        highlight(&state.editor.buffer().text, "py", "base16-ocean.dark");

    let mut draw = gfx.create_draw();
    draw.clear(convert_color(theme.settings.background.unwrap()));

    draw.text(&state.font, "0")
        .color(Color::TRANSPARENT)
        .size(state.line_height);
    let bounds = draw.last_text_bounds();
    let char_width = bounds.width;

    let cursor = state.editor.buffer().cursor;
    let cursor_line = state.editor.buffer().text.char_to_line(cursor);
    let cursor_line_position = state.editor.buffer().find_line_position(cursor);

    let line_count = state.editor.buffer().text.len_lines() - 1;
    let line_number_digit_count = line_count.to_string().len().max(3);
    let line_number_offset = if SHOW_LINE_NUMBERS {
        line_number_digit_count as f32 * char_width * 1.5
    } else {
        0.0
    };

    let camera_offset = calculate_camera_offset(
        cursor_line_position,
        cursor_line,
        char_width,
        state.line_height,
        gfx.size(),
    );

    // draw highlighted text
    for (index, line) in highlighted_lines.iter().enumerate() {
        let y_position = index as f32 * state.line_height;
        let mut char_index = 0usize;

        for (style, fragment) in line {
            let x_position = char_index as f32 * char_width;
            let text_position = (
                line_number_offset + camera_offset.0 + x_position,
                y_position + camera_offset.1,
            );
            draw.text(&state.font, &fragment)
                .position(text_position.0, text_position.1)
                .size(state.line_height)
                .color(convert_color(style.foreground));

            let word_length = fragment.chars().count();
            char_index += word_length;
        }
    }

    // render cursor
    {
        let x_position = char_width * cursor_line_position as f32;
        let y_position = state.line_height * cursor_line as f32;
        let cursor_color = convert_color(theme.settings.caret.unwrap());

        match state.editor.mode {
            Mode::Normal => {
                draw.rect(
                    (
                        x_position + line_number_offset + camera_offset.0,
                        y_position + camera_offset.1,
                    ),
                    (char_width, state.line_height),
                )
                .color(cursor_color);
            }
            Mode::Insert => {
                draw.line(
                    (
                        x_position + line_number_offset + camera_offset.0,
                        y_position + camera_offset.1,
                    ),
                    (
                        x_position + line_number_offset + camera_offset.0,
                        y_position + state.line_height + camera_offset.1,
                    ),
                )
                .color(cursor_color);
            }
            Mode::Command => {}
            Mode::QuickMenu => {}
        }
    }

    // render line number background
    let number_background_color = convert_color(theme.settings.background.unwrap());
    draw.rect(
        (0.0, 0.0),
        (
            line_number_digit_count as f32 * char_width + 2.0,
            gfx.size().1 as f32,
        ),
    )
    .color(number_background_color);

    // render line numbers
    for index in 0..line_count + 1 {
        let y_position = index as f32 * state.line_height;

        if SHOW_LINE_NUMBERS {
            // pad the line number with spaces on the left
            let line_number = format!(
                "{:>width$}",
                &index.to_string(),
                width = line_number_digit_count
            );

            // draw the line number
            draw.text(&state.font, &line_number)
                .position(0.0, y_position + camera_offset.1)
                .size(state.line_height)
                .color(Color::GRAY);
        }
    }

    // render command line at the bottom of the screen
    if state.editor.mode == Mode::Command {
        let (w, h) = gfx.size();
        draw.rect(
            (0.0, h as f32 - COMMAND_BOX_PADDING - state.line_height),
            (w as f32, h as f32),
        )
        .color(convert_color(theme.settings.background.unwrap()));

        draw.line(
            (0.0, h as f32 - COMMAND_BOX_PADDING - state.line_height),
            (w as f32, h as f32 - COMMAND_BOX_PADDING - state.line_height),
        )
        .color(convert_color(theme.settings.guide.unwrap()));

        draw.text(&state.font, &state.editor.command_line)
            .position(
                0.0,
                h as f32 - state.line_height - COMMAND_BOX_PADDING / 2.0,
            )
            .color(convert_color(theme.settings.foreground.unwrap()))
            .size(state.line_height);
    }

    if state.editor.mode == Mode::QuickMenu {
        // draw quick menu
        let margin_x = 80.0;
        let margin_y = 10.0;
        let width = gfx.size().0 as f32 - margin_x * 2.0;
        let height = gfx.size().1 as f32 - margin_y * 2.0;
        draw.rect((margin_x, margin_y), (width, height))
            .corner_radius(3.0)
            .stroke(4.0)
            .stroke_color(convert_color(theme.settings.guide.unwrap()))
            .fill()
            .fill_color(convert_color(theme.settings.background.unwrap()));
        draw.text(&state.font, &state.editor.quick_menu_line)
            .position(margin_x, margin_y)
            .color(convert_color(theme.settings.foreground.unwrap()));
    }

    gfx.render(&draw);
}

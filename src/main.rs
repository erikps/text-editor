use notan::draw::*;
use notan::prelude::*;

#[derive(Debug)]
struct Cursor {
    x: usize,
    y: usize,
}

#[derive(Debug, PartialEq)]
enum Mode {
    Normal,
    Input,
}

#[derive(AppState, Debug)]
struct State {
    font: Font,
    line_height: f32,

    cursor: Cursor,
    lines: Vec<String>,

    mode: Mode,

    last_time: f32,
}

impl State {
    pub fn move_x(&mut self, x: usize) {
        self.cursor.x = usize::min(
            usize::max(self.cursor.x + x, 0),
            self.lines[self.cursor.y].len() - 1,
        );
    }

    pub fn move_y(&mut self, y: usize) {
        self.cursor.y = usize::min(
            usize::max(self.cursor.y + y, 0),
            self.lines.len() - 1,
        );

        self.move_x(0);
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

    State {
        font,
        line_height: 16.0,

        cursor: Cursor { x: 0, y: 0 },
        lines,

        mode: Mode::Normal,

        last_time: 0.0,
    }
}

fn event(state: &mut State, event: Event) {
    if state.mode == Mode::Input {
        match event {
            Event::ReceivedCharacter(c) if c != '\u{7f}' && !c.is_control() && c.is_ascii() => {
                state.lines[state.cursor.y].insert(state.cursor.x, c);
                state.cursor.x += 1;
            }
            _ => {}
        }
    }
}

fn update(app: &mut App, state: &mut State) {
    let current_time = app.timer.elapsed_f32();

    match state.mode {
        Mode::Normal => {
            if app.keyboard.is_down(KeyCode::I) {
                state.mode = Mode::Input;
                return;
            }

            if current_time - state.last_time > INPUT_DELAY {
                if app.keyboard.is_down(KeyCode::J) {
                    // if state.cursor.y < state.lines.len() - 1 {
                    //     state.cursor.y = state.cursor.y + 1;
                    // }

                    // state.cursor.x =
                    //     usize::min(state.cursor.x, state.lines[state.cursor.y].len() - 1);
                    //
                    state.move_y(1);
                }

                if app.keyboard.is_down(KeyCode::K) {
                    if state.cursor.y > 0 {
                        state.cursor.y = state.cursor.y - 1;
                    }
                    state.cursor.x =
                        usize::min(state.cursor.x, state.lines[state.cursor.y].len() - 1);
                }

                if app.keyboard.is_down(KeyCode::L) {
                    if state.cursor.x < state.lines[state.cursor.y].len() - 1 {
                        state.cursor.x = state.cursor.x + 1;
                    }
                }

                if app.keyboard.is_down(KeyCode::H) {
                    if state.cursor.x > 0 {
                        state.cursor.x = state.cursor.x - 1;
                    }
                }

                state.last_time = current_time;
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

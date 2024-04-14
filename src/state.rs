use crate::buffer::{Cursor, Buffer};
use notan::prelude::{KeyCode, AppState};
use notan::draw::Font;
use crate::action::Action;
use crate::motion::Motion;
use std::collections::HashMap;

#[derive(PartialEq, Eq, Hash)]
pub struct Shortcut {
    pub key: KeyCode,
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
}

impl Shortcut {
    pub fn new(key: KeyCode) -> Self {
        Shortcut {
            key,
            ctrl: false,
            alt: false,
            shift: false,
        }
    }

    pub fn shift(mut self) -> Self {
        self.shift = true;
        self
    }

    pub fn ctrl(mut self) -> Self {
        self.ctrl = true;
        self
    }

    pub fn alt(&mut self) -> &mut Self {
        self.alt = true;
        self
    }
}

#[derive(PartialEq, Eq, Hash, Clone)]
pub enum ModeChange {
    Insert,
    InsertAfter,
    InsertEnd,
    InsertStart,
    Escape,
}

#[derive(Debug, PartialEq, Clone, Hash, Eq)]
pub enum Mode {
    Normal,
    Insert,
    Command,
}

pub type KeyBindings<T> = HashMap<Shortcut, T>;
pub type ActionBindings = KeyBindings<Action>;
pub type MotionBindings = KeyBindings<Motion>;
pub type ModeChangeBindings = KeyBindings<ModeChange>;

pub struct Keymap {
    pub action_bindings: ActionBindings,
    pub motion_bindings: MotionBindings,
    pub mode_change_bindings: HashMap<Mode, ModeChangeBindings>,
}

#[derive(AppState)]
pub struct State {
    pub font: Font,
    pub line_height: f32,

    pub buffer: Buffer,

    pub mode: Mode,

    pub action: Option<Action>,

    pub keymap: Keymap,

    pub last_time: f32,
    pub initial_movement_delay: f32,
    pub inter_movement_delay: f32,
}


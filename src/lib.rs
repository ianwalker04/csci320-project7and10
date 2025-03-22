#![no_std]

use num::Integer;
use pc_keyboard::{DecodedKey, KeyCode};
use pluggable_interrupt_os::vga_buffer::{
    is_drawable, plot, Color, ColorCode, plot_str
};

use core::{
    clone::Clone, cmp::{min, Eq, PartialEq}, iter::Iterator, marker::Copy, prelude::rust_2024::derive
};

const WINDOW_WIDTH: usize = 38;
const WINDOW_HEIGHT: usize = 10;
const WINDOW_1_START_COL: usize = 1;
const WINDOW_1_START_ROW: usize = 1;
const WINDOW_2_START_COL: usize = 41;
const WINDOW_2_START_ROW: usize = 1;
const WINDOW_3_START_COL: usize = 1;
const WINDOW_3_START_ROW: usize = 14;
const WINDOW_4_START_COL: usize = 41;
const WINDOW_4_START_ROW: usize = 14;

#[derive(Copy, Clone, Eq, PartialEq)]
pub struct SwimDocManager {
    documents: [SwimDocument; 4],
    active_window: usize
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub struct SwimDocument {
    letters: [[char; WINDOW_WIDTH]; WINDOW_HEIGHT],
    num_letters: usize,
    next_letter: usize,
    start_col: usize,
    start_row: usize,
    current_row: usize,
    cursor_position: usize,
    active: bool
}

pub fn safe_add<const LIMIT: usize>(a: usize, b: usize) -> usize {
    (a + b).mod_floor(&LIMIT)
}

pub fn add1<const LIMIT: usize>(value: usize) -> usize {
    safe_add::<LIMIT>(value, 1)
}

impl Default for SwimDocManager {
    fn default() -> Self {
        Self {
            documents: [SwimDocument::new(WINDOW_1_START_COL, WINDOW_1_START_ROW),
                        SwimDocument::new(WINDOW_2_START_COL, WINDOW_2_START_ROW),
                        SwimDocument::new(WINDOW_3_START_COL, WINDOW_3_START_ROW),
                        SwimDocument::new(WINDOW_4_START_COL, WINDOW_4_START_ROW)],
            active_window: 0
        }
    }
}

impl SwimDocManager {
    pub fn update(&mut self) {
        self.documents[self.active_window].active = true;
        for (i, mut doc) in self.documents.into_iter().enumerate() {
            if i != self.active_window {
                doc.active = false;
            }
            doc.tick();
        }
    }

    pub fn key(&mut self, key: DecodedKey) {
        match key {
            DecodedKey::RawKey(KeyCode::F1) => self.active_window = 0,
            DecodedKey::RawKey(KeyCode::F2) => self.active_window = 1,
            DecodedKey::RawKey(KeyCode::F3) => self.active_window = 2,
            DecodedKey::RawKey(KeyCode::F4) => self.active_window = 3,
            _ => {}
        }
        self.documents[self.active_window].key(key);
    }
}

impl SwimDocument {
    pub fn new(start_col: usize, start_row: usize) -> Self {
        Self {
            letters: [['\0'; WINDOW_WIDTH]; WINDOW_HEIGHT],
            num_letters: 0,
            next_letter: 0,
            start_col,
            start_row,
            current_row: 0,
            cursor_position: 0,
            active: false
        }
    }

    fn letter_columns(&self) -> impl Iterator<Item = usize> + '_ {
        0..self.num_letters
    }

    pub fn tick(&mut self) {
        self.clear_current();
        self.draw_current();
        self.draw_outline(self.active);
    }

    fn clear_current(&self) {
        let row: usize = self.get_actual_row();
        for col in self.letter_columns() {
            let actual_col: usize = self.start_col + col;
            plot(' ', actual_col, row, ColorCode::new(Color::Black, Color::Black));
        }
        plot(' ', self.start_col + self.cursor_position, row, ColorCode::new(Color::Black, Color::Black));
    }

    fn draw_current(&mut self) {
        let row: usize = self.get_actual_row();
        for (i, _) in self.letter_columns().enumerate() {
            let actual_col: usize = self.start_col + i;
            plot(
                self.letters[self.current_row][i],
                actual_col,
                row,
                ColorCode::new(Color::White, Color::Black),
            );
        }
        plot(' ', self.start_col + self.cursor_position, row, ColorCode::new(Color::White, Color::White));
    }

    fn draw_outline(&self, active: bool) {
        let color: ColorCode;
        if active {
            color = ColorCode::new(Color::Black, Color::White);
        } else {
            color = ColorCode::new(Color::White, Color::Black);
        }
        for col in self.start_col - 1..=self.start_col + WINDOW_WIDTH {
            plot('*', col, self.start_row - 1, color);
            plot('*', col, self.start_row + WINDOW_HEIGHT, color);
        }
        for row in self.start_row - 1..=self.start_row + WINDOW_HEIGHT {
            plot('*', self.start_col - 1, row, color);
            plot('*', self.start_col + WINDOW_WIDTH, row, color);
        }
        plot_str("F1", 19, 0, ColorCode::new(Color::White, Color::Black));
        plot_str("F2", 59, 0, ColorCode::new(Color::White, Color::Black));
        plot_str("F3", 19, 13, ColorCode::new(Color::White, Color::Black));
        plot_str("F4", 59, 13, ColorCode::new(Color::White, Color::Black));
    }

    fn get_actual_row(&self) -> usize {
        self.start_row + (self.current_row % WINDOW_HEIGHT)
    }

    fn start_new_line(&mut self) {
        let row: usize = self.get_actual_row();
        plot(' ', self.start_col + self.cursor_position, row, ColorCode::new(Color::Black, Color::Black));
        self.current_row = (self.current_row + 1) % WINDOW_HEIGHT;
        self.cursor_position = 0;
        self.num_letters = 0;
        self.next_letter = 0;
    }

    pub fn key(&mut self, key: DecodedKey) {
        match key {
            DecodedKey::RawKey(_) => {},
            DecodedKey::Unicode(char) => self.handle_unicode(char)
        }
    }

    fn handle_unicode(&mut self, key: char) {
        if key == '\n' {
            self.start_new_line();
        } else if is_drawable(key) {
            if self.cursor_position >= WINDOW_WIDTH - 1 {
                let current_char: char = key;
                self.start_new_line();
                self.letters[self.current_row][self.next_letter] = current_char;
            } else {
                self.letters[self.current_row][self.next_letter] = key;
            }
            self.next_letter = add1::<WINDOW_WIDTH>(self.next_letter);
            self.num_letters = min(self.num_letters + 1, WINDOW_WIDTH);
            self.cursor_position = add1::<WINDOW_WIDTH>(self.cursor_position);
        }
    }
}

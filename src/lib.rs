#![no_std]

use num::Integer;
use pc_keyboard::{DecodedKey, KeyCode};
use pluggable_interrupt_os::vga_buffer::{
    is_drawable, plot, Color, ColorCode, BUFFER_HEIGHT, BUFFER_WIDTH,
};

use core::{
    clone::Clone, cmp::{min, Eq, PartialEq}, iter::Iterator, marker::Copy, prelude::rust_2024::derive
};

#[derive(Copy, Clone, Eq, PartialEq)]
pub struct SwimInterface {
    letters: [[char; BUFFER_WIDTH]; BUFFER_HEIGHT],
    num_letters: usize,
    next_letter: usize,
    col: usize,
    row: usize,
    cursor_position: usize
}

pub fn safe_add<const LIMIT: usize>(a: usize, b: usize) -> usize {
    (a + b).mod_floor(&LIMIT)
}

pub fn add1<const LIMIT: usize>(value: usize) -> usize {
    safe_add::<LIMIT>(value, 1)
}

pub fn sub1<const LIMIT: usize>(value: usize) -> usize {
    safe_add::<LIMIT>(value, LIMIT - 1)
}

impl Default for SwimInterface {
    fn default() -> Self {
        Self {
            letters: [['\0'; BUFFER_WIDTH]; BUFFER_HEIGHT],
            num_letters: 0,
            next_letter: 0,
            col: 0,
            row: 0,
            cursor_position: 0
        }
    }
}

impl SwimInterface {
    fn letter_columns(&self) -> impl Iterator<Item = usize> + '_ {
        0..self.num_letters
    }

    pub fn tick(&mut self) {
        self.clear_current();
        self.draw_current();
    }

    fn clear_current(&self) {
        for col in self.letter_columns() {
            plot(' ', col, self.row, ColorCode::new(Color::Black, Color::Black));
        }
        plot(' ', self.cursor_position, self.row, ColorCode::new(Color::Black, Color::Black));
    }

    fn draw_current(&mut self) {
        for (i, _) in self.letter_columns().enumerate() {
            plot(
                self.letters[self.row][i],
                i,
                self.row,
                ColorCode::new(Color::White, Color::Black),
            );
        }
        plot(' ', self.cursor_position, self.row, ColorCode::new(Color::White, Color::White));
    }

    pub fn start_new_line(&mut self) {
        plot(' ', self.cursor_position, self.row, ColorCode::new(Color::Black, Color::Black));
        self.row = add1::<BUFFER_HEIGHT>(self.row);
        self.cursor_position = 0;
        self.num_letters = 0;
        self.next_letter = 0;
    }

    pub fn key(&mut self, key: DecodedKey) {
        match key {
            DecodedKey::RawKey(code) => self.handle_raw(code),
            DecodedKey::Unicode(c) => self.handle_unicode(c)
        }
    }

    fn handle_raw(&mut self, key: KeyCode) {
        match key {
            _ => {}
        }
    }

    fn handle_unicode(&mut self, key: char) {
        if key == '\n' {
            self.start_new_line();
        } else if is_drawable(key) {
            if self.cursor_position >= BUFFER_WIDTH - 1 {
                let current_char: char = key;
                self.start_new_line();
                self.letters[self.row][self.next_letter] = current_char;
            } else {
                self.letters[self.row][self.next_letter] = key;
            }
            self.next_letter = add1::<BUFFER_WIDTH>(self.next_letter);
            self.num_letters = min(self.num_letters + 1, BUFFER_WIDTH);
            self.cursor_position = add1::<BUFFER_WIDTH>(self.cursor_position);
        }
    }
}

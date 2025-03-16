#![no_std]

use num::Integer;
use pc_keyboard::{DecodedKey, KeyCode};
use pluggable_interrupt_os::vga_buffer::{
    is_drawable, plot, Color, ColorCode, BUFFER_HEIGHT, BUFFER_WIDTH,
};

use core::{
    clone::Clone,
    cmp::{min, Eq, PartialEq},
    iter::Iterator,
    marker::Copy,
    prelude::rust_2024::derive,
};

#[derive(Copy, Clone, Eq, PartialEq)]
pub struct SwimInterface {
    letters: [char; BUFFER_WIDTH],
    num_letters: usize,
    next_letter: usize,
    col: usize,
    row: usize,
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
            letters: ['A'; BUFFER_WIDTH],
            num_letters: 1,
            next_letter: 1,
            col: BUFFER_WIDTH / 2,
            row: BUFFER_HEIGHT / 2,
        }
    }
}

impl SwimInterface {
    fn letter_columns(&self) -> impl Iterator<Item = usize> + '_ {
        (0..self.num_letters).map(|n| safe_add::<BUFFER_WIDTH>(n, self.col))
    }

    pub fn tick(&mut self) {
        self.clear_current();
        self.draw_current();
    }

    fn clear_current(&self) {
        for x in self.letter_columns() {
            plot(' ', x, self.row, ColorCode::new(Color::Black, Color::Black));
        }
    }

    fn draw_current(&self) {
        for (i, x) in self.letter_columns().enumerate() {
            plot(
                self.letters[i],
                x,
                self.row,
                ColorCode::new(Color::Cyan, Color::Black),
            );
        }
    }

    pub fn key(&mut self, key: DecodedKey) {
        match key {
            DecodedKey::RawKey(code) => self.handle_raw(code),
            DecodedKey::Unicode(c) => self.handle_unicode(c),
        }
    }

    fn handle_raw(&mut self, key: KeyCode) {
        match key {
            _ => {}
        }
    }

    fn handle_unicode(&mut self, key: char) {
        if is_drawable(key) {
            self.letters[self.next_letter] = key;
            self.next_letter = add1::<BUFFER_WIDTH>(self.next_letter);
            self.num_letters = min(self.num_letters + 1, BUFFER_WIDTH);
        }
    }
}

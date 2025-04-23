#![no_std]

use file_system_solution::FileSystem;
use ramdisk::RamDisk;
use num::Integer;
use pc_keyboard::{DecodedKey, KeyCode};
use pluggable_interrupt_os::vga_buffer::{
    is_drawable, plot, Color, ColorCode, plot_str, BUFFER_WIDTH
};
use core::cmp::min;

// Window Constraints
const WINDOW_WIDTH: usize = (WIN_REGION_WIDTH - 3) / 2;
const WINDOW_HEIGHT: usize = 10;
const WINDOW_1_START_COL: usize = 1;
const WINDOW_1_START_ROW: usize = 1;
const WINDOW_2_START_COL: usize = 36;
const WINDOW_2_START_ROW: usize = 1;
const WINDOW_3_START_COL: usize = 1;
const WINDOW_3_START_ROW: usize = 14;
const WINDOW_4_START_COL: usize = 36;
const WINDOW_4_START_ROW: usize = 14;

// File System Constraints
const TASK_MANAGER_WIDTH: usize = 10;
const WIN_REGION_WIDTH: usize = BUFFER_WIDTH - TASK_MANAGER_WIDTH;
const MAX_OPEN: usize = 16;
const BLOCK_SIZE: usize = 256;
const NUM_BLOCKS: usize = 255;
const MAX_FILE_BLOCKS: usize = 64;
const MAX_FILE_BYTES: usize = MAX_FILE_BLOCKS * BLOCK_SIZE;
const MAX_FILES_STORED: usize = 30;
const MAX_FILENAME_BYTES: usize = 10;

pub struct SwimDocManager {
    documents: [SwimDocument; 4],
    active_window: usize
}

pub struct SwimDocument {
    letters: [[char; WINDOW_WIDTH]; WINDOW_HEIGHT],
    num_letters: usize,
    next_letter: usize,
    start_col: usize,
    start_row: usize,
    current_row: usize,
    cursor_position: usize,
    active: bool,
    file_system: FileSystem<MAX_OPEN, BLOCK_SIZE, NUM_BLOCKS, MAX_FILE_BLOCKS, MAX_FILE_BYTES, MAX_FILES_STORED, MAX_FILENAME_BYTES>,
    window_status: WindowStatus,
    active_file: usize
}

#[derive(PartialEq)]
enum WindowStatus {
    DisplayingFiles,
    EditingFile,
    ExecutingFile
}

fn safe_add<const LIMIT: usize>(a: usize, b: usize) -> usize {
    (a + b).mod_floor(&LIMIT)
}

fn add1<const LIMIT: usize>(value: usize) -> usize {
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
        for (i, doc) in self.documents.iter_mut().enumerate() {
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
    fn new(start_col: usize, start_row: usize) -> Self {
        let mut swim_doc: SwimDocument = Self {
            letters: [['\0'; WINDOW_WIDTH]; WINDOW_HEIGHT],
            num_letters: 0,
            next_letter: 0,
            start_col,
            start_row,
            current_row: 0,
            cursor_position: 0,
            active: false,
            file_system: FileSystem::new(RamDisk::new()),
            window_status: WindowStatus::DisplayingFiles,
            active_file: 0
        };
        swim_doc.create_default_files();
        swim_doc
    }

    fn create_default_files(&mut self) {
        let hello: usize = self.file_system.open_create("hello").unwrap();
        self.file_system.write(hello, r#"print("Hello, world!")"#.as_bytes()).unwrap();
        self.file_system.close(hello).unwrap();
        let nums: usize = self.file_system.open_create("nums").unwrap();
        self.file_system.write(nums, r#"print(1)
print(257)"#.as_bytes()).unwrap();
        self.file_system.close(nums).unwrap();
        let average: usize = self.file_system.open_create("average").unwrap();
        self.file_system.write(average, r#"sum := 0
count := 0
averaging := true
while averaging {
    num := input("Enter a number:")
    if (num == "quit") {
        averaging := false
    } else {
        sum := (sum + num)
        count := (count + 1)
    }
}
print((sum / count))"#.as_bytes()).unwrap();
        self.file_system.close(average).unwrap();
        let pi: usize = self.file_system.open_create("pi").unwrap();
        self.file_system.write(pi, r#"sum := 0
i := 0
neg := false
terms := input("Num terms:")
while (i < terms) {
    term := (1.0 / ((2.0 * i) + 1.0))
    if neg {
        term := -term
    }
    sum := (sum + term)
    neg := not neg
    i := (i + 1)
}
print((4 * sum))"#.as_bytes()).unwrap();
        self.file_system.close(pi).unwrap();
    }

    fn display_files(&mut self) {
        let files: (usize, [[u8; 10]; 30]) = self.file_system.list_directory().unwrap();
        let mut col: usize = self.start_col;
        let mut row: usize = self.start_row - 1;
        for file_num in 0..files.0 {
            let text: &str = str::from_utf8(&files.1[file_num]).unwrap();
            if file_num % 3 == 0 {
                col = self.start_col;
                row += 1;
            } else {
                col += 10;
            }
            if file_num == self.active_file {
                plot_str(text, col, row, ColorCode::new(Color::Black, Color::White));
            } else {
                plot_str(text, col, row, ColorCode::new(Color::White, Color::Black));
            }
        }
    }

    fn letter_columns(&self) -> impl Iterator<Item = usize> + '_ {
        0..self.num_letters
    }

    fn tick(&mut self) {
        self.clear_current();
        self.draw_outline(self.active);
        if self.window_status == WindowStatus::DisplayingFiles {
            self.display_files();
        }
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
        if self.window_status == WindowStatus::EditingFile {
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
        plot_str("F1", 16, 0, ColorCode::new(Color::White, Color::Black));
        plot_str("F2", 52, 0, ColorCode::new(Color::White, Color::Black));
        plot_str("F3", 16, 13, ColorCode::new(Color::White, Color::Black));
        plot_str("F4", 52, 13, ColorCode::new(Color::White, Color::Black));
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

    fn key(&mut self, key: DecodedKey) {
        match key {
            DecodedKey::RawKey(KeyCode::ArrowLeft) => {
                if !self.active {
                    return;
                }
                if self.window_status != WindowStatus::DisplayingFiles {
                    return;
                }
                if self.active_file > 0 {
                    self.active_file -= 1;
                }
            },
            DecodedKey::RawKey(KeyCode::ArrowRight) => {
                if !self.active {
                    return;
                }
                if self.window_status != WindowStatus::DisplayingFiles {
                    return;
                }
                let num_files: usize = self.file_system.list_directory().unwrap().0;
                if self.active_file < num_files - 1 {
                    self.active_file += 1;
                }
            },
            DecodedKey::Unicode(char) => self.handle_unicode(char),
            DecodedKey::RawKey(_) => {},
        }
    }

    fn handle_unicode(&mut self, key: char) {
        if self.window_status == WindowStatus::EditingFile {
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
}

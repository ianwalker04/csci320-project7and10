#![no_std]

use file_system_solution::FileSystem;
use gc_heap_template::GenerationalHeap;
use ramdisk::RamDisk;
use num::Integer;
use pc_keyboard::{DecodedKey, KeyCode};
use pluggable_interrupt_os::vga_buffer::{
    is_drawable, plot, Color, ColorCode, plot_str, plot_num, BUFFER_WIDTH
};
use core::cmp::min;
use core::str;
use simple_interp::{Interpreter, InterpreterOutput, ArrayString};

// Window Constants
const WINDOW_WIDTH: usize = (WIN_REGION_WIDTH - 3) / 2;
const WINDOW_HEIGHT: usize = 10;
const WINDOW_1_START_COL: usize = 1;
const WINDOW_1_START_ROW: usize = 2;
const WINDOW_2_START_COL: usize = 36;
const WINDOW_2_START_ROW: usize = 2;
const WINDOW_3_START_COL: usize = 1;
const WINDOW_3_START_ROW: usize = 14;
const WINDOW_4_START_COL: usize = 36;
const WINDOW_4_START_ROW: usize = 14;

// File System Constants
const TASK_MANAGER_WIDTH: usize = 10;
const WIN_REGION_WIDTH: usize = BUFFER_WIDTH - TASK_MANAGER_WIDTH;
const MAX_OPEN: usize = 16;
const BLOCK_SIZE: usize = 256;
const NUM_BLOCKS: usize = 255;
const MAX_FILE_BLOCKS: usize = 64;
const MAX_FILE_BYTES: usize = MAX_FILE_BLOCKS * BLOCK_SIZE;
const MAX_FILES_STORED: usize = 30;
const MAX_FILENAME_BYTES: usize = 10;

// Program Execution Constants
const MAX_TOKENS: usize = 100;
const MAX_LITERAL_CHARS: usize = 15;
const STACK_DEPTH: usize = 20;
const MAX_LOCAL_VARS: usize = 10;
const HEAP_SIZE: usize = 256;
const MAX_HEAP_BLOCKS: usize = HEAP_SIZE;
pub struct SwimDocManager {
    documents: [SwimDocument; 4],
    interpreters: [Option<Interpreter<MAX_TOKENS, MAX_LITERAL_CHARS, STACK_DEPTH, MAX_LOCAL_VARS, WINDOW_WIDTH, GenerationalHeap<HEAP_SIZE, MAX_HEAP_BLOCKS, 2>>>; 4],
    active_window: usize,
    f1_ticks: usize,
    f2_ticks: usize,
    f3_ticks: usize,
    f4_ticks: usize
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
    active_file: usize,
    program_running: bool
}

#[derive(PartialEq)]
enum WindowStatus {
    DisplayingFiles,
    EditingFile,
    ExecutingFile,
    AwaitingInput,
    DisplayingOutput
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
            interpreters: [None; 4],
            active_window: 0,
            f1_ticks: 0,
            f2_ticks: 0,
            f3_ticks: 0,
            f4_ticks: 0
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
            if doc.program_running && doc.window_status != WindowStatus::AwaitingInput {
                match i {
                    0 => self.f1_ticks += 1,
                    1 => self.f2_ticks += 1,
                    2 => self.f3_ticks += 1,
                    3 => self.f4_ticks += 1,
                    _ => {}
                }
            }
            doc.tick(&mut self.interpreters[i]);
        }
        self.draw_program_ticks();
    }

    pub fn key(&mut self, key: DecodedKey) {
        match key {
            DecodedKey::RawKey(KeyCode::F1) => self.active_window = 0,
            DecodedKey::RawKey(KeyCode::F2) => self.active_window = 1,
            DecodedKey::RawKey(KeyCode::F3) => self.active_window = 2,
            DecodedKey::RawKey(KeyCode::F4) => self.active_window = 3,
            DecodedKey::RawKey(KeyCode::F6) => {
                let active_doc: &mut SwimDocument = &mut self.documents[self.active_window];
                active_doc.clear_window();
                active_doc.window_status = WindowStatus::DisplayingFiles;
            },
            DecodedKey::Unicode(char) => {
                if char == 'r' {
                    let active_doc: &mut SwimDocument = &mut self.documents[self.active_window];
                    if active_doc.window_status != WindowStatus::DisplayingFiles {
                        return;
                    }
                    if active_doc.window_status == WindowStatus::DisplayingOutput {
                        active_doc.clear_window();
                        active_doc.window_status = WindowStatus::DisplayingFiles;
                        return;
                    }
                    let files: [[u8; 10]; 30] = active_doc.file_system.list_directory().unwrap().1;
                    let file_name: &str = str::from_utf8(&files[active_doc.active_file]).unwrap().trim_matches(char::from(0));
                    let fd: usize = active_doc.file_system.open_read(file_name.trim()).unwrap();
                    let mut buffer: [u8; MAX_FILE_BYTES] = [0; MAX_FILE_BYTES];
                    active_doc.file_system.read(fd, &mut buffer).unwrap();
                    let file: &str = str::from_utf8(&buffer).unwrap().trim_matches(char::from(0));
                    active_doc.file_system.close(fd).unwrap();
                    active_doc.window_status = WindowStatus::ExecutingFile;
                    active_doc.program_running = true;
                    active_doc.clear_window();
                    self.interpreters[self.active_window] = Some(Interpreter::new(file));
                }
            }
            _ => {}
        }
        self.documents[self.active_window].key(key);
    }

    fn draw_program_ticks(&self) {
        plot_str("F1", 71, 0, ColorCode::new(Color::White, Color::Black));
        plot_num(self.f1_ticks as isize, 71, 1, ColorCode::new(Color::White, Color::Black));
        plot_str("F2", 71, 2, ColorCode::new(Color::White, Color::Black));
        plot_num(self.f2_ticks as isize, 71, 3, ColorCode::new(Color::White, Color::Black));
        plot_str("F3", 71, 4, ColorCode::new(Color::White, Color::Black));
        plot_num(self.f3_ticks as isize, 71, 5, ColorCode::new(Color::White, Color::Black));
        plot_str("F4", 71, 6, ColorCode::new(Color::White, Color::Black));
        plot_num(self.f4_ticks as isize, 71, 7, ColorCode::new(Color::White, Color::Black));
    }
}

impl InterpreterOutput for SwimDocument {
    fn print(&mut self, chars: &[u8]) {
        self.program_running = false;
        let output: &str = str::from_utf8(chars).unwrap().trim();
        // panic!("{output}");
        plot_str(output, self.start_col, self.start_row, ColorCode::new(Color::White, Color::Black));
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
            active_file: 0,
            program_running: false
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
            let text: &str = str::from_utf8(&files.1[file_num]).unwrap().trim_matches(char::from(0));
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

    fn tick(&mut self, interpreter: &mut Option<Interpreter<MAX_TOKENS, MAX_LITERAL_CHARS, STACK_DEPTH, MAX_LOCAL_VARS, WINDOW_WIDTH, GenerationalHeap<HEAP_SIZE, MAX_HEAP_BLOCKS, 2>>>) {
        self.draw_outline();
        if self.window_status == WindowStatus::DisplayingFiles {
            self.display_files();
        }
        if self.window_status == WindowStatus::ExecutingFile {
            match interpreter {
                Some(mut ip) => {
                    match ip.tick(self) {
                        simple_interp::TickStatus::Continuing => {
                            // panic!("Continuing");
                        },
                        simple_interp::TickStatus::Finished => {
                            panic!("Output should be displayed");
                            self.window_status = WindowStatus::DisplayingOutput;
                            *interpreter = None;
                        },
                        simple_interp::TickStatus::AwaitInput => {
                            self.window_status = WindowStatus::AwaitingInput;
                            plot_str("Awaiting input", 10, 10, ColorCode::new(Color::White, Color::Black));
                        }
                    }
                },
                None => {}
            }
        }
    }

    fn clear_window(&self) {
        for row in self.start_row..self.start_row + WINDOW_HEIGHT {
            for col in self.start_col..self.start_col + WINDOW_WIDTH {
                plot(' ', col, row, ColorCode::new(Color::Black, Color::Black));
            }
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

    fn draw_outline(&self) {
        let color: ColorCode;
        if self.active {
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
        plot_str("F1", 16, 1, ColorCode::new(Color::White, Color::Black));
        plot_str("F2", 52, 1, ColorCode::new(Color::White, Color::Black));
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

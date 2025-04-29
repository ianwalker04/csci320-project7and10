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
const MAX_FILES_STORED: usize = 31;
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
    f4_ticks: usize,
    next_tick: usize,
    creating_file: bool,
    new_filename: [char; MAX_FILENAME_BYTES],
    new_filename_length: usize
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
    program_running: bool,
    output_line: usize,
    array_string: ArrayString<WINDOW_WIDTH>,
    current_editing_file: [u8; MAX_FILENAME_BYTES],
    current_editing_file_len: usize,
    input_row: usize
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
            f4_ticks: 0,
            next_tick: 0,
            creating_file: false,
            new_filename: ['\0'; MAX_FILENAME_BYTES],
            new_filename_length: 0
        }
    }
}

impl SwimDocManager {
    pub fn update(&mut self) {
        if self.creating_file {
            plot_str("Filename: ", 0, 0, ColorCode::new(Color::White, Color::Black));
            for i in 0..self.new_filename_length {
                plot(self.new_filename[i], 10 + i, 0, ColorCode::new(Color::White, Color::Black));
            }
            plot(' ', 10 + self.new_filename_length, 0, ColorCode::new(Color::White, Color::White));
        }
        for i in 0..self.documents.len() {
            self.documents[i].active = i == self.active_window;
            self.documents[i].draw_outline();
            if self.documents[i].window_status == WindowStatus::DisplayingFiles {
                self.documents[i].display_files();
            }
            if self.documents[i].window_status == WindowStatus::AwaitingInput {
                self.documents[i].clear_line(self.documents[i].start_row + 1);
                self.documents[i].draw_current(1);
            }
        }
        let mut running_programs: [usize; 4] = [0; 4];
        let mut count: usize = 0;
        for i in 0..self.documents.len() {
            if self.documents[i].program_running &&
               self.documents[i].window_status != WindowStatus::AwaitingInput {
                if count < running_programs.len() {
                    running_programs[count] = i;
                    count += 1;
                }
            }
        }
        if count > 0 {
            let doc_to_tick: usize = running_programs[self.next_tick % count];
            match doc_to_tick {
                0 => self.f1_ticks += 1,
                1 => self.f2_ticks += 1,
                2 => self.f3_ticks += 1,
                3 => self.f4_ticks += 1,
                _ => {}
            }
            self.documents[doc_to_tick].tick(&mut self.interpreters[doc_to_tick]);
            self.next_tick = (self.next_tick + 1) % count;
        }
        self.draw_program_ticks();
    }

    pub fn key(&mut self, key: DecodedKey) {
        if self.creating_file {
            self.file_creation_input(key);
            return;
        }
        match key {
            DecodedKey::RawKey(KeyCode::F1) => self.active_window = 0,
            DecodedKey::RawKey(KeyCode::F2) => self.active_window = 1,
            DecodedKey::RawKey(KeyCode::F3) => self.active_window = 2,
            DecodedKey::RawKey(KeyCode::F4) => self.active_window = 3,
            DecodedKey::RawKey(KeyCode::F5) => {
                self.creating_file = true;
                self.new_filename = ['\0'; MAX_FILENAME_BYTES];
                self.new_filename_length = 0;
                for col in 0..WIN_REGION_WIDTH {
                    plot(' ', col, 0, ColorCode::new(Color::Black, Color::Black));
                }
            },
            DecodedKey::RawKey(KeyCode::F6) => {
                let mut save: bool = false;
                let mut filename: [u8; MAX_FILENAME_BYTES] = [0u8; MAX_FILENAME_BYTES];
                let mut filename_len: usize = 0;
                let mut buffer: [u8; MAX_FILE_BYTES] = [0; MAX_FILE_BYTES];
                let mut buffer_position: usize = 0;
                {
                    let active_doc: &mut SwimDocument = &mut self.documents[self.active_window];
                    
                    if active_doc.window_status == WindowStatus::EditingFile && active_doc.current_editing_file_len > 0 {
                        save = true;
                        filename_len = active_doc.current_editing_file_len;
                        for i in 0..filename_len {
                            filename[i] = active_doc.current_editing_file[i];
                        }
                        for row in 0..WINDOW_HEIGHT {
                            if !active_doc.is_line_empty(row) {
                                for col in 0..active_doc.get_line_length(row) {
                                    if buffer_position >= MAX_FILE_BYTES - 2 {
                                        break;
                                    }
                                    buffer[buffer_position] = active_doc.letters[row][col] as u8;
                                    buffer_position += 1;
                                }
                                if buffer_position < MAX_FILE_BYTES - 2 {
                                    let mut next_non_empty_row: usize = row + 1;
                                    while next_non_empty_row < WINDOW_HEIGHT && 
                                        active_doc.is_line_empty(next_non_empty_row) {
                                        next_non_empty_row += 1;
                                    }
                                    if next_non_empty_row < WINDOW_HEIGHT {
                                        buffer[buffer_position] = b'\n';
                                        buffer_position += 1;
                                    }
                                }
                            }
                        }
                    }
                    active_doc.clear_window();
                    active_doc.program_running = false;
                    active_doc.window_status = WindowStatus::DisplayingFiles;
                }
                if save {
                    if let Ok(active_filename) = str::from_utf8(&filename[0..filename_len]) {
                        let filename: &str = active_filename.trim_matches(char::from(0));
                        for doc in self.documents.iter_mut() {
                            if let Ok(fd) = doc.file_system.open_create(filename) {
                                doc.file_system.write(fd, &buffer[0..buffer_position]).unwrap();
                                doc.file_system.close(fd).unwrap();
                            }
                        }
                    }
                }
            },
            DecodedKey::Unicode(char) => {
                let active_doc: &mut SwimDocument = &mut self.documents[self.active_window];
                if active_doc.window_status == WindowStatus::DisplayingFiles {
                    if char == 'e' {
                        let active_doc: &mut SwimDocument = &mut self.documents[self.active_window];
                        if active_doc.window_status != WindowStatus::DisplayingFiles {
                            return;
                        }
                        let files: [[u8; 10]; MAX_FILES_STORED] = active_doc.file_system.list_directory().unwrap().1;
                        active_doc.current_editing_file_len = 0;
                        for &byte in files[active_doc.active_file].iter() {
                            if byte == 0 {
                                break;
                            }
                            active_doc.current_editing_file[active_doc.current_editing_file_len] = byte;
                            active_doc.current_editing_file_len += 1;
                        }
                        let file_name: &str = str::from_utf8(&active_doc.current_editing_file[0..active_doc.current_editing_file_len]).unwrap().trim_matches(char::from(0));
                        let fd: usize = active_doc.file_system.open_read(file_name).unwrap();
                        let mut buffer: [u8; MAX_FILE_BYTES] = [0; MAX_FILE_BYTES];
                        active_doc.file_system.read(fd, &mut buffer).unwrap();
                        let file_content: &str = str::from_utf8(&buffer).unwrap().trim_matches(char::from(0));
                        active_doc.file_system.close(fd).unwrap();
                        active_doc.window_status = WindowStatus::EditingFile;
                        active_doc.clear_window();
                        for row in 0..WINDOW_HEIGHT {
                            for col in 0..WINDOW_WIDTH {
                                active_doc.letters[row][col] = '\0';
                            }
                        }
                        let mut row: usize = 0;
                        let mut col: usize = 0;
                        for char in file_content.chars() {
                            if char == '\n' {
                                for i in 0..col {
                                    plot(
                                        active_doc.letters[row][i],
                                        active_doc.start_col + i,
                                        active_doc.start_row + row,
                                        ColorCode::new(Color::White, Color::Black),
                                    );
                                }
                                row += 1;
                                col = 0;
                                if row >= WINDOW_HEIGHT {
                                    break;
                                }
                            } else if is_drawable(char) {
                                if col < WINDOW_WIDTH {
                                    active_doc.letters[row][col] = char;
                                    col += 1;
                                }
                            }
                        }
                        if row < WINDOW_HEIGHT {
                            for i in 0..col {
                                plot(
                                    active_doc.letters[row][i],
                                    active_doc.start_col + i,
                                    active_doc.start_row + row,
                                    ColorCode::new(Color::White, Color::Black),
                                );
                            }
                        }
                        active_doc.current_row = 0;
                        active_doc.cursor_position = 0;
                        let first_line_length: usize = col;
                        active_doc.num_letters = first_line_length;
                        active_doc.next_letter = first_line_length;
                        plot(' ', 
                            active_doc.start_col + active_doc.cursor_position,
                            active_doc.start_row + active_doc.current_row, 
                            ColorCode::new(Color::White, Color::White));
                        return;
                    }
                    if char == 'r' {
                        let active_doc: &mut SwimDocument = &mut self.documents[self.active_window];
                        if active_doc.window_status != WindowStatus::DisplayingFiles {
                            return;
                        }
                        if active_doc.window_status == WindowStatus::DisplayingOutput {
                            active_doc.clear_window();
                            active_doc.program_running = false;
                            active_doc.window_status = WindowStatus::DisplayingFiles;
                            return;
                        }
                        let files: [[u8; 10]; MAX_FILES_STORED] = active_doc.file_system.list_directory().unwrap().1;
                        let file_name: &str = str::from_utf8(&files[active_doc.active_file]).unwrap().trim_matches(char::from(0));
                        let fd: usize = active_doc.file_system.open_read(file_name.trim()).unwrap();
                        let mut buffer: [u8; MAX_FILE_BYTES] = [0; MAX_FILE_BYTES];
                        active_doc.file_system.read(fd, &mut buffer).unwrap();
                        let file: &str = str::from_utf8(&buffer).unwrap().trim_matches(char::from(0));
                        active_doc.file_system.close(fd).unwrap();
                        active_doc.window_status = WindowStatus::ExecutingFile;
                        active_doc.clear_window();
                        active_doc.output_line = 0;
                        active_doc.current_row = 0;
                        active_doc.cursor_position = 0;
                        active_doc.num_letters = 0;
                        active_doc.next_letter = 0;
                        active_doc.program_running = true;
                        self.interpreters[self.active_window] = Some(Interpreter::new(file));
                    }
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

    fn file_creation_input(&mut self, key: DecodedKey) {
        match key {
            DecodedKey::Unicode('\n') => {
                if self.new_filename_length > 0 {
                    let mut filename_bytes: [u8; 10] = [0u8; MAX_FILENAME_BYTES];
                    for i in 0..self.new_filename_length {
                        filename_bytes[i] = self.new_filename[i] as u8;
                    }
                    let filename: &str = str::from_utf8(&filename_bytes[0..self.new_filename_length]).unwrap();
                    for doc in self.documents.iter_mut() {
                        let fd: usize;
                        match doc.file_system.open_create(filename) {
                            Ok(value) => fd = value,
                            Err(_) => {
                                plot_str("Too many files!", 20, 0, ColorCode::new(Color::White, Color::Black));
                                return;
                            }
                        }
                        doc.file_system.close(fd).unwrap();
                    }
                    self.creating_file = false;
                    for col in 0..WIN_REGION_WIDTH {
                        plot(' ', col, 0, ColorCode::new(Color::Black, Color::Black));
                    }
                }
            },
            DecodedKey::Unicode('\u{8}') => {
                if self.new_filename_length > 0 {
                    for i in 0..=self.new_filename_length {
                        plot(' ', 10 + i, 0, ColorCode::new(Color::Black, Color::Black));
                    }
                    self.new_filename_length -= 1;
                    self.new_filename[self.new_filename_length] = '\0';
                    for i in 0..self.new_filename_length {
                        plot(self.new_filename[i], 10 + i, 0, ColorCode::new(Color::White, Color::Black));
                    }
                    plot(' ', 10 + self.new_filename_length, 0, ColorCode::new(Color::White, Color::White));
                }
            },
            DecodedKey::Unicode(char) => {
                if is_drawable(char) && self.new_filename_length < MAX_FILENAME_BYTES - 1 {
                    self.new_filename[self.new_filename_length] = char;
                    self.new_filename_length += 1;
                    plot(char, 10 + self.new_filename_length - 1, 0, ColorCode::new(Color::White, Color::Black));
                }
            },
            _ => {}
        }
    }
}

impl InterpreterOutput for SwimDocument {
    fn print(&mut self, chars: &[u8]) {
        let output: &str = str::from_utf8(chars).unwrap().trim();
        if self.output_line >= WINDOW_HEIGHT {
            for row in 0..WINDOW_HEIGHT-1 {
                self.clear_line(self.start_row + row);
            }
            self.output_line = WINDOW_HEIGHT - 1;
        }
        self.clear_line(self.start_row + self.output_line);
        plot_str(output, self.start_col, self.start_row + self.output_line, 
                 ColorCode::new(Color::White, Color::Black));
        self.output_line += 1;
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
            program_running: false,
            output_line: 0,
            array_string: ArrayString::default(),
            current_editing_file: [0; MAX_FILENAME_BYTES],
            current_editing_file_len: 0,
            input_row: 0
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
        let files: (usize, [[u8; 10]; MAX_FILES_STORED]) = self.file_system.list_directory().unwrap();
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
        if self.window_status == WindowStatus::ExecutingFile {
            match interpreter {
                Some(ref mut ip) => {
                    if let Ok(input_str) = self.array_string.as_str() {
                        if !input_str.is_empty() {
                            ip.provide_input(input_str).unwrap();
                            self.array_string.clear();
                            self.clear_line(self.start_row);
                        }
                    }
                    match ip.tick(self) {
                        simple_interp::TickStatus::Continuing => {},
                        simple_interp::TickStatus::Finished => {
                            self.window_status = WindowStatus::DisplayingOutput;
                            self.program_running = false;
                            *interpreter = None;
                        },
                        simple_interp::TickStatus::AwaitInput => {
                            self.window_status = WindowStatus::AwaitingInput;
                            self.clear_line(self.start_row + 1);
                            self.current_row = 0;
                            self.cursor_position = 0;
                            self.num_letters = 0;
                            self.next_letter = 0;
                        }
                    }
                },
                None => {}
            }
        }
        if self.window_status == WindowStatus::AwaitingInput {
            self.clear_current(1);
            self.draw_current(1);
            self.output_line = 0;
        }
    }

    fn clear_window(&self) {
        for row in self.start_row..self.start_row + WINDOW_HEIGHT {
            for col in self.start_col..self.start_col + WINDOW_WIDTH {
                plot(' ', col, row, ColorCode::new(Color::Black, Color::Black));
            }
        }
    }

    fn clear_current(&self, offset: usize) {
        let row: usize = self.get_actual_row() + offset;
        for col in self.letter_columns() {
            let actual_col: usize = self.start_col + col;
            plot(' ', actual_col, row, ColorCode::new(Color::Black, Color::Black));
        }
        plot(' ', self.start_col + self.cursor_position, row, ColorCode::new(Color::Black, Color::Black));
    }

    fn clear_line(&self, row: usize) {
        for col in self.start_col..self.start_col + WINDOW_WIDTH {
            plot(' ', col, row, ColorCode::new(Color::Black, Color::Black));
        }
    }

    fn draw_current(&mut self, offset: usize) {
        let row: usize = self.get_actual_row() + offset;
        let buffer_row: usize = if self.window_status == WindowStatus::AwaitingInput {
            self.input_row
        } else {
            self.current_row
        };
        for (i, _) in self.letter_columns().enumerate() {
            let actual_col: usize = self.start_col + i;
            plot(
                self.letters[buffer_row][i],
                actual_col,
                row,
                ColorCode::new(Color::White, Color::Black),
            );
        }
        plot(' ', self.start_col + self.cursor_position, row, ColorCode::new(Color::White, Color::White));
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
        let window_label: &str = match (self.start_col, self.start_row) {
            (1, 2) => "F1",
            (36, 2) => "F2",
            (1, 14) => "F3",
            (36, 14) => "F4",
            _ => "",
        };
        plot_str(window_label, self.start_col, self.start_row - 1, ColorCode::new(Color::White, Color::Black));
        if self.window_status == WindowStatus::EditingFile && self.current_editing_file_len > 0 {
            let label_offset = window_label.len();
            if let Ok(filename) = str::from_utf8(&self.current_editing_file[0..self.current_editing_file_len]) {
                plot_str(filename, self.start_col + label_offset + 1, self.start_row - 1, 
                        ColorCode::new(Color::White, Color::Black));
            }
        }
    }

    fn get_actual_row(&self) -> usize {
        self.start_row + (self.current_row % WINDOW_HEIGHT)
    }

    fn start_new_line(&mut self, offset: usize) {
        let row: usize = self.get_actual_row() + offset;
        plot(' ', self.start_col + self.cursor_position, row, ColorCode::new(Color::Black, Color::Black));
        self.current_row = (self.current_row + 1) % (WINDOW_HEIGHT - offset);
        self.cursor_position = 0;
        self.num_letters = 0;
        self.next_letter = 0;
    }

    fn get_line_length(&self, row: usize) -> usize {
        let mut length: usize = 0;
        for i in 0..WINDOW_WIDTH {
            if self.letters[row][i] == '\0' {
                break;
            }
            length += 1;
        }
        length
    }
    
    fn is_line_empty(&self, row: usize) -> bool {
        self.letters[row][0] == '\0'
    }

    fn draw_all_lines(&self) {
        for row in 0..WINDOW_HEIGHT {
            if !self.is_line_empty(row) {
                for col in 0..self.get_line_length(row) {
                    plot(
                        self.letters[row][col],
                        self.start_col + col,
                        self.start_row + row,
                        ColorCode::new(Color::White, Color::Black),
                    );
                }
            }
        }
        plot(' ', 
            self.start_col + self.cursor_position,
            self.start_row + self.current_row, 
            ColorCode::new(Color::White, Color::White)
        );
    }

    fn key(&mut self, key: DecodedKey) {
        match key {
            DecodedKey::RawKey(KeyCode::ArrowUp) => {
                if !self.active {
                    return;
                }
                if self.window_status == WindowStatus::EditingFile {
                    if self.current_row > 0 {
                        plot(' ', 
                            self.start_col + self.cursor_position,
                            self.start_row + self.current_row, 
                            ColorCode::new(Color::Black, Color::Black)
                        );
                        self.current_row -= 1;
                        let line_length: usize = self.get_line_length(self.current_row);
                        self.cursor_position = core::cmp::min(self.cursor_position, line_length);
                        self.num_letters = line_length;
                        self.next_letter = line_length;
                        self.draw_all_lines();
                    }
                }
            },
            DecodedKey::RawKey(KeyCode::ArrowDown) => {
                if !self.active {
                    return;
                }
                if self.window_status == WindowStatus::EditingFile {
                    if self.current_row < WINDOW_HEIGHT - 1 && !self.is_line_empty(self.current_row + 1) {
                        plot(' ', 
                            self.start_col + self.cursor_position,
                            self.start_row + self.current_row, 
                            ColorCode::new(Color::Black, Color::Black)
                        );
                        self.current_row += 1;
                        let line_length: usize = self.get_line_length(self.current_row);
                        self.cursor_position = core::cmp::min(self.cursor_position, line_length);
                        self.num_letters = line_length;
                        self.next_letter = line_length;
                        self.draw_all_lines();
                    }
                }
            },
            DecodedKey::RawKey(KeyCode::ArrowLeft) => {
                if !self.active {
                    return;
                }
                if self.window_status == WindowStatus::DisplayingFiles {
                    if self.active_file > 0 {
                        self.active_file -= 1;
                    }
                } else if self.window_status == WindowStatus::EditingFile {
                    if self.cursor_position > 0 {
                        self.clear_line(self.get_actual_row());
                        self.cursor_position -= 1;
                        self.draw_current(0);
                    }
                }
            },
            DecodedKey::RawKey(KeyCode::ArrowRight) => {
                if !self.active {
                    return;
                }
                if self.window_status == WindowStatus::DisplayingFiles {
                    let num_files: usize = self.file_system.list_directory().unwrap().0;
                    if self.active_file < num_files - 1 {
                        self.active_file += 1;
                    }
                } else if self.window_status == WindowStatus::EditingFile {
                    if self.cursor_position < self.num_letters {
                        self.cursor_position += 1;
                        self.draw_current(0);
                    }
                }
            },
            DecodedKey::Unicode('\u{8}') => {
                if self.window_status == WindowStatus::AwaitingInput || 
                   self.window_status == WindowStatus::EditingFile {
                    self.handle_unicode('\u{8}');
                }
            },
            DecodedKey::Unicode(char) => {
                if self.window_status == WindowStatus::AwaitingInput ||
                   self.window_status == WindowStatus::EditingFile {
                    self.handle_unicode(char);
                }
            },
            DecodedKey::RawKey(_) => {},
        }
    }

    fn handle_unicode(&mut self, key: char) {
        if key == '\n' {
            if self.window_status == WindowStatus::AwaitingInput {
                let mut input_string: ArrayString<33> = ArrayString::default();
                for i in 0..self.num_letters {
                    input_string.push_char(self.letters[self.input_row][i]);
                }
                self.cursor_position = 0;
                self.num_letters = 0;
                self.next_letter = 0;
                self.window_status = WindowStatus::ExecutingFile;
                self.program_running = true;
                self.array_string = input_string;
            } else {
                self.start_new_line(0);
            }
        } else if key == '\u{8}' {
            if self.cursor_position > 0 {
                let row_to_use: usize = if self.window_status == WindowStatus::AwaitingInput {
                    self.input_row
                } else {
                    self.current_row
                };
                for i in self.cursor_position-1..self.num_letters-1 {
                    self.letters[row_to_use][i] = self.letters[row_to_use][i+1];
                }
                self.letters[row_to_use][self.num_letters-1] = '\0';
                self.num_letters -= 1;
                self.next_letter = self.num_letters;
                self.cursor_position -= 1;
                self.clear_line(self.get_actual_row() + 
                    (if self.window_status == WindowStatus::AwaitingInput { 1 } else { 0 }));
                self.draw_current(if self.window_status == WindowStatus::AwaitingInput { 1 } else { 0 });
            }
        } else if is_drawable(key) {
            let row_to_use: usize = if self.window_status == WindowStatus::AwaitingInput {
                self.input_row
            } else {
                self.current_row
            };
            self.letters[row_to_use][self.cursor_position] = key;
            self.next_letter = min(add1::<WINDOW_WIDTH>(self.next_letter), WINDOW_WIDTH - 1);
            self.num_letters = min(self.num_letters + 1, WINDOW_WIDTH);
            self.cursor_position = min(add1::<WINDOW_WIDTH>(self.cursor_position), WINDOW_WIDTH - 1);
            self.clear_line(self.get_actual_row() + 
                (if self.window_status == WindowStatus::AwaitingInput { 1 } else { 0 }));
                self.draw_current(if self.window_status == WindowStatus::AwaitingInput { 1 } else { 0 });
        }
    }
}

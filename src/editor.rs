use crate::Document;
use crate::Row;
use crate::Terminal;
use std::env; 
use std::cmp;
use std::time::Duration;
use std::time::Instant;
use termion::event::Key;
use termion::color;

const STATUS_BG_COLOR: color::Rgb = color::Rgb(239, 239, 239);
const STATUS_FG_COLOR: color::Rgb = color::Rgb(63, 63, 63);
const VERSION: &str = env!("CARGO_PKG_VERSION");
const QUIT_TIMES: u8 = 3;

#[derive(Default)]
pub struct Position {
    pub x: usize,
    pub y: usize,
}

struct StatusMessage {
    text: String,
    time: Instant,
}

impl StatusMessage {
    fn from(msg: String) -> Self {
        Self {
            time: Instant::now(),
            text: msg,
        }
    }
}
pub struct Editor {
    quit: bool,
    terminal: Terminal,
    cursor_position: Position,
    offset: Position,
    document: Document,
    status_msg: StatusMessage,
    quit_times: u8,
}

impl Editor {
    pub fn default() -> Self {
        let args: Vec<String> = env::args().collect();
        let mut inital_status = 
            String::from("HELP: Ctrs-f = find | Ctrl-s = save | Ctrl-q = quit");
        let document = if let Some(file_name) = args.get(1) {
            let doc = Document::open(file_name);
            if let Ok(doc) = doc {
                doc
            } else {
                inital_status = format!("ERR: Could not open file: {}", file_name);
                Document::default()
            }
        } else {
            Document::default()
        };
        Self{ 
            quit: false,
            terminal: Terminal::default().expect("Failed to initialize terminal"),
            document,
            cursor_position: Position::default(),
            offset: Position::default(),
            status_msg: StatusMessage::from(inital_status),
            quit_times: QUIT_TIMES,
        }
    }

    pub fn run(&mut self) {

        loop {
            if let Err(error) = self.refresh_screen() {
                die(&error);
            }
            if self.quit {
                break;
            }
            if let Err(error) = self.process_keypress() {
                die(&error);
            }
        }
    }

    fn refresh_screen(&self) -> Result<(), std::io::Error> {
        Terminal::cursor_hide();
        Terminal::cursor_position(&Position::default());
        if self.quit {
            Terminal::clear_screen();
            println!("Adios. \r");
        } else {
            self.draw_rows();
            self.draw_status_bar();
            self.draw_msg_bar();
            Terminal::cursor_position(&Position {
                x: self.cursor_position.x.saturating_sub(self.offset.x),
                y: self.cursor_position.y.saturating_sub(self.offset.y),

            });
        }
        Terminal::cursor_show();
        Terminal::flush()
    }

    fn save(&mut self) {
        if self.document.name.is_none() {
            let new_name = self.prompt("Save as: ").unwrap_or(None);
            if new_name.is_none() {
                self.status_msg = StatusMessage::from("Save aborted.".to_string());
                return;
            }
            self.document.name = new_name;
        }
        if self.document.save().is_ok() {
            self.status_msg = StatusMessage::from("File saved.".to_string());
        } else {
            self.status_msg = StatusMessage::from("Error writing file!".to_string());
        }

    }

    fn process_keypress(&mut self) -> Result<(), std::io::Error> {
        let pressed_key = Terminal::read_key()?;
        match pressed_key {
            Key::Ctrl('q') => {
                if self.quit_times > 0 && self.document.is_dirty() {
                    self.status_msg = StatusMessage::from(format!(
                            "WARNING! Changies will be lost. Press Ctrl-q {} more times to quit", 
                            self.quit_times));
                    self.quit_times -= 1;
                    return Ok(());
                }
                self.quit = true;
            }
            Key::Ctrl('f') => {
                if let Some(query) = self.prompt("Search: ").unwrap_or(None) {
                    if let Some(position) = self.document.find(&query[..]) {
                        self.cursor_position = position;
                    } else {
                        self.status_msg = StatusMessage::from(format!("Not found :{}.",query));
                    }
                }
            }
            Key::Ctrl('s') => self.save(),
            Key::Char(c) => {
                self.document.insert(&self.cursor_position, c);
                self.move_cursor(Key::Right);
            }
            Key::Delete => self.document.delete(&self.cursor_position),
            Key::Backspace => {
                if self.cursor_position.x > 0 || self.cursor_position.y > 0 {
                    self.move_cursor(Key::Left);
                    self.document.delete(&self.cursor_position);
                }
            }
            Key::Up 
            | Key::Down 
            | Key::Left 
            | Key::Right 
            | Key::PageUp
            | Key::PageDown
            | Key::End
            | Key::Home => self.move_cursor(pressed_key),
            _ => (),
        }
        self.scroll();
        if self.quit_times < QUIT_TIMES {
            self.quit_times = QUIT_TIMES;
            self.status_msg = StatusMessage::from(String::new());
        }
        Ok(())
    }

    fn scroll(&mut self) {
        let Position { x, y } = self.cursor_position;
        let width = self.terminal.size().width as usize;
        let height = self.terminal.size().height as usize;
        let mut offset = &mut self.offset;
        if y < offset.y {
            offset.y = y;
        } else if y >= offset.y.saturating_add(height) {
            offset.y = y.saturating_sub(height).saturating_add(1);
        }
        if x < offset.x {
            offset.x = x;
        } else if x >= offset.x.saturating_add(width) {
            offset.x = x.saturating_sub(width).saturating_add(1);
        }
    }

    pub fn draw_row(&self, row: &Row) {
        let width = self.terminal.size().width as usize;
        let start = self.offset.x;
        let end = self.offset.x.saturating_add(width);
        let row = row.render(start, end);
        println!("{}\r", row)
    }

    #[allow(clippy::integer_arithmetic, clippy::integer_division)]
    fn draw_rows(&self) {
        let height = self.terminal.size().height - 1;
        for terminal_row in 0..height {
            Terminal::clear_current_line();
            if let Some(row) = self
                .document
                .row(self.offset.y.saturating_add(terminal_row as usize)) {
                self.draw_row(row);
            } else if self.document.is_empty() && terminal_row == height / 3 {
                self.draw_welcome();
            } else {
                println!("~\r");
            }
        }
    }

    fn draw_welcome(&self) {
        let mut welcome_msg = format!("RustTe editor -- version {}\r", VERSION);
        let width = self.terminal.size().width as usize;
        let len = welcome_msg.len();
        #[allow(clippy::integer_arithmetic, clippy::integer_division)]
        let padding = width.saturating_sub(len) / 2;
        let spaces = " ".repeat(padding.saturating_sub(1));
        welcome_msg = format!("~{}{}", spaces, welcome_msg);
        welcome_msg.truncate(width);
        println!("{}\r", welcome_msg);

    }
    
    fn draw_status_bar(&self) {
        let mut status;
        let width = self.terminal.size().width as usize;
        let mod_indicator = if self.document.is_dirty() {
            " (modified)"
        } else {
            ""
        };
        let mut file_name = "[Unnamed]".to_string();
        if let Some(name) = &self.document.name {
            file_name = name.clone();
            file_name.truncate(20);
        }
        status = format!("{} - {} lines{}", file_name, self.document.len(), mod_indicator);
        let line_indicator = format!(
            "{}/{}",
            self.cursor_position.y.saturating_add(1),
            self.document.len()
        );
        #[allow(clippy::integer_arithmetic)]
        let len = status.len() + line_indicator.len();
        status.push_str(&" ".repeat(width.saturating_sub(len)));
        status = format!("{}{}", status, line_indicator);
        status.truncate(width);
        Terminal::set_bg_color(STATUS_BG_COLOR);
        Terminal::set_fg_color(STATUS_FG_COLOR);
        println!("{}\r",status);
        Terminal::reset_bg_color();
        Terminal::reset_fg_color();
    }

    fn draw_msg_bar(&self) {
        Terminal::clear_current_line();
        let msg = &self.status_msg;
        if Instant::now() - msg.time < Duration::new(5, 0) {
            let mut text = msg.text.clone();
            text.truncate(self.terminal.size().width as usize);
            print!("{}", text);
        }
    }

    fn move_cursor(&mut self, key: Key) {
        let terminal_height = self.terminal.size().height as usize;
        let Position { mut x, mut y} = self.cursor_position;
        let height = self.document.len();
        let mut width = if let Some(row) = self.document.row(y) {
            row.len()
        } else {
            0
        };
        match key {
            Key::Up => y = y.saturating_sub(1),
            Key::Down => {
                if y < height {
                    y = y.saturating_add(1);
                }
            }
            Key::Left => {
                if x > 0 {
                    x -= 1;
                } else if y > 0 {
                    y -= 1;
                    if let Some(row) = self.document.row(y) {
                        x = row.len();
                    } else {
                        x = 0;
                    }
                }
            }
            Key::Right => {
                if x < width {
                    x += 1;
                } else if y < height {
                    y += 1;
                    x = 0;
                 }
            }
            Key::PageUp => {
                y = if y > terminal_height {
                    y.saturating_sub(terminal_height)
                } else {
                    0
                }
            }
            Key::PageDown => {
                y = if y.saturating_add(terminal_height) < height {
                    y.saturating_sub(terminal_height)
                } else {
                    height
                }
            }
            Key::Home => x = 0,
            Key::End => x = width,
            _ => (),
        }
        width = if let Some(row) = self.document.row(y) {
            row.len()
        } else {
            0
        };
        x = cmp::min(x, width);

        self.cursor_position = Position { x, y }
    }

    fn prompt(&mut self, prompt: &str) -> Result<Option<String>, std::io::Error> {
        let mut result = String::new();
        loop {
            self.status_msg = StatusMessage::from(format!("{}{}", prompt, result));
            self.refresh_screen()?;
            match   Terminal::read_key()? {
                Key::Backspace => result.truncate(result.len().saturating_sub(1)),
                Key::Char('\n') => break,
                Key::Char(c) => {
                    if !c.is_control() {
                        result.push(c);
                    }
                }
                Key::Esc => {
                    result.truncate(0);
                    break;
                }
                _ => (),
            }
        }
        self.status_msg = StatusMessage::from(String::new());
        if result.is_empty() {
            return Ok(None);
        }
        Ok(Some(result))
    }
}

fn die(e: &std::io::Error) {
    Terminal::clear_screen();
    panic!("{}", e);
}

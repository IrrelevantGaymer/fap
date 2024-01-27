use std::{
    io::{self, ErrorKind}, 
    path::{Path, PathBuf}, 
    cmp::{max, min}, 
    fs::DirEntry, process::Stdio,
    fmt::Display
};

use crossterm::{
    cursor::{MoveDown, MoveLeft, MoveRight, MoveTo, MoveToColumn, MoveToRow, MoveUp, RestorePosition, SavePosition},
    event::{read, Event, KeyCode, KeyEvent},
    terminal::{
        self, Clear, ClearType, SetSize
    },
    execute, style::{Color, ResetColor, SetForegroundColor, Stylize}
};
use is_executable::IsExecutable;
use path_absolutize::Absolutize;
use unicode_segmentation::UnicodeSegmentation;

use crate::{BOTTOM_RESERVED, START_X, START_Y, Position, write_to_screen, writeln_to_screen};

pub struct App {
    pub buffer: Vec<(Option<PathBuf>, usize, String)>,
    pub cd: PathBuf,
    pub output: String,
    pub index: u16,
    pub stored_position: Position,
    pub cursor_position: Position,
    pub command_state: CommandState
}

pub struct CommandState {
    number: Option<u16>,
    prefix: Prefix,
}

#[allow(non_camel_case_types)]
#[allow(dead_code)]
#[derive(PartialEq)]
pub enum Prefix {
    f,
    F,
    g,
    t,
    T,
    z,
    None
}

impl App {
    pub fn generate_buffer(&mut self){
        let mut output = vec![];
        output.push((None, 48, format!("{empty:=<48}", empty = "")));
        output.push((
            None, 
            self.cd.to_str().unwrap_or_default().len(), 
            format!("{}", self.cd.to_str().unwrap())
        ));
        output.push((None, 48, format!("{empty:=<48}", empty = "")));
        match self.cd.parent() {
            Some(parent) => output.push((
                Some(parent.to_path_buf()), 
                3,
                format!("{}..{}/{}", 
                    SetForegroundColor(Color::Cyan), 
                    SetForegroundColor(Color::Rgb {r: 255, g: 192, b: 203}),
                    ResetColor
                )
            )),
            None => ()
        };
        output.push((None, 2, format!("{}.{}/{}", 
            SetForegroundColor(Color::Cyan), 
            SetForegroundColor(Color::Rgb {r: 255, g: 192, b: 203}),
            ResetColor
        )));
        let dir = &self.cd;
        let dir_entries: Vec<DirEntry> = std::fs::read_dir(dir)
            .unwrap().map(|e| e.unwrap()).collect();
    
        let (mut dirs, mut files) = (vec![], vec![]);
        for dir_entry in dir_entries {
            if dir_entry.metadata().unwrap().is_dir() {
                dirs.push(dir_entry);
            } else {
                files.push(dir_entry);
            }
        } 
    
        dirs.sort_by_key(|dir| dir.path());
        files.sort_by_key(|dir| dir.path());
        
        for path in dirs {
            output.push((
                Some(path.path()), 
                path.file_name().to_str().unwrap_or_default().graphemes(true).count() + 1,
                format!("{}{}", path.file_name().to_str().unwrap().stylize().cyan(), "/".stylize().red())
            ));
        }
        for path in files {
            output.push((
                Some(path.path()), 
                path.file_name().to_str().unwrap_or_default().graphemes(true).count(),
                path.file_name().to_str().unwrap_or_default().to_string()
            ));
        }
    
        self.buffer = output;
    }

    pub fn draw_screen(&self) -> io::Result<()> {
        
        execute!(io::stderr(), Clear(ClearType::All), MoveTo(0, 0))?;
    
        let rows = terminal::window_size()?.rows;
    
        for i in 0..rows-BOTTOM_RESERVED {
            if i as usize >= self.buffer.len() {
                break;
            }
            let (_, _, display) = &self.buffer[(self.index + i) as usize];
            writeln_to_screen(format!("{}", display))?;
        }
    
        self.write_bottom()?;
    
        return Ok(());
    }

    pub fn write_bottom(&self) -> io::Result<()> {
        let rows = terminal::window_size()?.rows;
    
        let (_, len, _) = &self.buffer[(self.index + self.cursor_position.row()) as usize];
        
        execute!(io::stderr(), MoveToColumn(0), MoveToRow(rows - BOTTOM_RESERVED))?;
        writeln_to_screen(format!("{empty:=<24}", empty = ""))?;
        writeln_to_screen(format!(
            "x: {}, y: {}. cur item len: {} {empty: <8}", 
            self.cursor_position.col(), 
            self.cursor_position.row(), 
            len,
            empty = ""
        ))?;
        write_to_screen(format!("{}", self.command_state))?;
    
        return Ok(());
    }

    pub fn read_input(&mut self) -> io::Result<()> {
        self.cursor_position = Position(START_X, START_Y);
        self.stored_position = Position(START_X, START_Y);
        loop {
            match read()? {
                Event::Key(KeyEvent{code: KeyCode::Char(n), ..}) if n.is_digit(10) => {
                    if n == '0' && self.command_state.number.is_none() {

                        continue;
                    }
                    self.command_state.push(n.to_digit(10).unwrap());
                }
                Event::Key(KeyEvent{code: KeyCode::Char('g'), ..}) => {
                    if self.command_state.prefix == Prefix::g {
                        self.loop_fn(
                            |s| s.move_cursor_to_first_line()
                        )?;
                        self.command_state.prefix = Prefix::None;
                    } else {
                        self.command_state.prefix = Prefix::g;
                    }
                },
                Event::Key(KeyEvent{code: KeyCode::Char('G'), ..}) => {
                    self.move_cursor_to_last_line()?;
                    self.command_state.prefix = Prefix::None;
                },
                Event::Key(KeyEvent{code: KeyCode::Char('H'), ..}) => {
                    self.move_cursor_to_top()?;
                    self.command_state.prefix = Prefix::None;
                },
                Event::Key(KeyEvent{code: KeyCode::Char('M'), ..}) => {
                    self.move_cursor_to_middle()?;
                    self.command_state.prefix = Prefix::None;
                },
                Event::Key(KeyEvent{code: KeyCode::Char('L'), ..}) => {
                    self.move_cursor_to_bottom()?;
                    self.command_state.prefix = Prefix::None;
                },
                Event::Key(KeyEvent{code: KeyCode::Char('h'), ..}) => {
                    self.loop_fn(
                        |s| s.move_cursor_left()
                    )?;
                    self.command_state.prefix = Prefix::None;
                },
                Event::Key(KeyEvent{code: KeyCode::Char('j'), ..}) => {
                    self.loop_fn(
                        |s| s.move_cursor_down()
                    )?;
                    self.command_state.prefix = Prefix::None;
                },
                Event::Key(KeyEvent{code: KeyCode::Char('k'), ..}) => {
                    self.loop_fn(
                        |s| s.move_cursor_up()
                    )?;
                    self.command_state.prefix = Prefix::None;
                },
                Event::Key(KeyEvent{code: KeyCode::Char('l'), ..}) => {
                    self.loop_fn(
                        |s| s.move_cursor_right()
                    )?;
                    self.command_state.prefix = Prefix::None;
                },
                Event::Key(KeyEvent{code: KeyCode::Char(' '), ..}) => {
                    self.output = self.cd.to_str().ok_or(
                        io::Error::new(ErrorKind::InvalidData, "cannot parse cd")
                    )?.to_string();
                    break;
                },
                Event::Key(KeyEvent{code: KeyCode::Enter, ..}) => {
                    self.select_entry()?;
                    self.command_state.prefix = Prefix::None;
                },
                Event::Key(KeyEvent{code: KeyCode::Esc, ..}) => {
                    self.cd = Path::new("./").to_path_buf();
                    break;
                },
                Event::Resize(width, height) => {
                    self.window_resize(width, height)?;
                }
                _ => ()
            }
            execute!(io::stderr(), SavePosition)?;
            self.write_bottom()?;
            execute!(io::stderr(), RestorePosition)?;
        }
    
        return Ok(());
    }

    pub fn loop_fn(&mut self, fun: fn(&mut Self) -> io::Result<()>) -> io::Result<()> {
        let times = self.command_state.number.unwrap_or(1);
        for _ in 0..times {
            fun(self)?;
        }
        self.command_state.number = None;

        return Ok(());
    }
    
    pub fn move_cursor_to_first_line(&mut self) -> io::Result<()> {
        let (col, _) = self.cursor_position.get();
    
        let (_, len, _) = self.buffer[0];
        let max_col = max(len - 1, 0) as u16;
        
        execute!(io::stderr(), MoveToRow(0))?;
        self.index = 0;
        self.stored_position.set_row(0);
        self.cursor_position.set_row(0);
    
        if max_col < col {
            execute!(io::stderr(), MoveToColumn(max_col))?;
            self.cursor_position.set_col(max_col);
        } else if col < min(self.stored_position.col(), max_col) {
            let new_col = min(self.stored_position.col(), max_col);
            execute!(io::stderr(), MoveToColumn(new_col))?;
            self.cursor_position.set_col(new_col);
        }
        execute!(io::stderr(), SavePosition)?;
        self.draw_screen()?;
        execute!(io::stderr(), RestorePosition)?;
    
        return Ok(());
    }
    
    pub fn move_cursor_to_last_line(&mut self) -> io::Result<()> {
        let (col, _) = self.cursor_position.get();
        let height = terminal::window_size()?.rows - BOTTOM_RESERVED - 1;
    
        if (self.buffer.len() as u16) < height {
            self.index = 0;
            self.stored_position.set_row((self.buffer.len() - 1) as u16);
        } else {
            self.index = (self.buffer.len() as u16) - height - 1;
            self.stored_position.set_row(height);
        }
    
        let (_, len, _) = self.buffer[self.buffer.len() - 1];
        let max_col = max(len - 1, 0) as u16;
        
        execute!(io::stderr(), MoveToRow(self.stored_position.row()))?;
        self.cursor_position.set_row(self.stored_position.row());
    
        if max_col < col {
            execute!(io::stderr(), MoveToColumn(max_col))?;
            self.cursor_position.set_col(max_col);
        } else if col < min(self.stored_position.col(), max_col) {
            let new_col = min(self.stored_position.col(), max_col);
            execute!(io::stderr(), MoveToColumn(new_col))?;
            self.cursor_position.set_col(new_col);
        }
        execute!(io::stderr(), SavePosition)?;
        self.draw_screen()?;
        execute!(io::stderr(), RestorePosition)?;
    
        return Ok(());
    }
    
    pub fn move_cursor_to_top(&mut self) -> io::Result<()> {
        let (col, _) = self.cursor_position.get();
    
        let (_, len, _) = self.buffer[self.index as usize];
        let max_col = max(len - 1, 0) as u16;
        
        execute!(io::stderr(), MoveToRow(0))?;
        self.stored_position.set_row(0);
        self.cursor_position.set_row(0);
    
        if max_col < col {
            execute!(io::stderr(), MoveToColumn(max_col))?;
            self.cursor_position.set_col(max_col);
        } else if col < min(self.stored_position.col(), max_col) {
            let new_col = min(self.stored_position.col(), max_col);
            execute!(io::stderr(), MoveToColumn(new_col))?;
            self.cursor_position.set_col(new_col);
        }
        execute!(io::stderr(), SavePosition)?;
        self.write_bottom()?;
        execute!(io::stderr(), RestorePosition)?;
        
        return Ok(());
    }
    
    pub fn move_cursor_to_middle(&mut self) -> io::Result<()> {
        let (col, _) = self.cursor_position.get();
        let midpoint = (terminal::window_size()?.rows - BOTTOM_RESERVED - 1) / 2;
        let i = min((self.index + midpoint) as usize, self.buffer.len() - 1) as u16;
        let row = i - self.index;
    
        let (_, len, _) = self.buffer[i as usize];
        let max_col = max(len - 1, 0) as u16;
        
        execute!(io::stderr(), MoveToRow(row))?;
        self.stored_position.set_row(row);
        self.cursor_position.set_row(row);
    
        if max_col < col {
            execute!(io::stderr(), MoveToColumn(max_col))?;
            self.cursor_position.set_col(max_col);
        } else if col < min(self.stored_position.col(), max_col) {
            let new_col = min(self.stored_position.col(), max_col);
            execute!(io::stderr(), MoveToColumn(new_col))?;
            self.cursor_position.set_col(new_col);
        }
        execute!(io::stderr(), SavePosition)?;
        self.write_bottom()?;
        execute!(io::stderr(), RestorePosition)?;
        
        return Ok(());
    }
    
    pub fn move_cursor_to_bottom(&mut self) -> io::Result<()> {
        let (col, _) = self.cursor_position.get();
        let bottom = terminal::window_size()?.rows - BOTTOM_RESERVED - 1;
        let i = min((self.index + bottom) as usize, self.buffer.len() - 1) as u16;
        let row = i - self.index;
    
        let (_, len, _) = self.buffer[i as usize];
        let max_col = max(len - 1, 0) as u16;
        
        execute!(io::stderr(), MoveToRow(row))?;
        self.stored_position.set_row(row);
        self.cursor_position.set_row(row);
    
        if max_col < col {
            execute!(io::stderr(), MoveToColumn(max_col))?;
            self.cursor_position.set_col(max_col);
        } else if col < min(self.stored_position.col(), max_col) {
            let new_col = min(self.stored_position.col(), max_col);
            execute!(io::stderr(), MoveToColumn(new_col))?;
            self.cursor_position.set_col(new_col);
        }
        execute!(io::stderr(), SavePosition)?;
        self.write_bottom()?;
        execute!(io::stderr(), RestorePosition)?;
        
        return Ok(());
    }
    
    pub fn move_cursor_left(&mut self) -> io::Result<()> {
        let (col, _) = self.cursor_position.get();
        if col <= 0 {
            return Ok(());
        }
        execute!(io::stderr(), MoveLeft(1), SavePosition)?;
        self.stored_position.set_col(col - 1);
        self.cursor_position.move_left();
        self.write_bottom()?;
        execute!(io::stderr(), RestorePosition)?;
        return Ok(());
    }
    
    pub fn move_cursor_down(&mut self) -> io::Result<()> {
        let rows = terminal::window_size()?.rows;
    
        let (col, row) = self.cursor_position.get();
    
        if (self.index + self.stored_position.row()) as usize >= self.buffer.len() - 1 {
            return Ok(());
        }
    
        let (_, len, _) = self.buffer[(self.index + row + 1) as usize];
        let max_col = max(len - 1, 0) as u16;
    
        if self.stored_position.row() >= rows - BOTTOM_RESERVED - 1 {
            self.index += 1;
            execute!(io::stderr(), SavePosition)?;
            self.draw_screen()?;
            execute!(io::stderr(), RestorePosition)?;
        } else {
            execute!(io::stderr(), MoveDown(1))?;
            self.stored_position.set_row(self.stored_position.row() + 1);
            self.cursor_position.move_down();
        }
    
        if max_col < col {
            execute!(io::stderr(), MoveToColumn(max_col))?;
            self.cursor_position.set_col(max_col);
        } else if col < min(self.stored_position.col(), max_col) {
            let new_col = min(self.stored_position.col(), max_col);
            execute!(io::stderr(), MoveToColumn(new_col))?;
            self.cursor_position.set_col(new_col);
        }
    
        execute!(io::stderr(), SavePosition)?;
        self.write_bottom()?;
        execute!(io::stderr(), RestorePosition)?;
    
        return Ok(());
    }
    
    pub fn move_cursor_up(&mut self) -> io::Result<()> {
        let (col, row) = self.cursor_position.get();
    
        if self.index + row == 0 {
            return Ok(());
        }
    
        let (_, len, _) = self.buffer[(self.index + row - 1) as usize];
        let max_col = max(len - 1, 0) as u16;
        
        if self.stored_position.row() == 0 {
            if self.index > 0 {
                self.index -= 1;
                execute!(io::stderr(), SavePosition)?;
                self.draw_screen()?;
                execute!(io::stderr(), RestorePosition)?;
            }
        } else {
            execute!(io::stderr(), MoveUp(1))?;
            self.stored_position.set_row(self.stored_position.row() - 1);
            self.cursor_position.move_up();
        }
    
        if max_col < col {
            execute!(io::stderr(), MoveToColumn(max_col))?;
            self.cursor_position.set_col(max_col);
        } else if col < min(self.stored_position.col(), max_col) {
            let new_col = min(self.stored_position.col(), max_col);
            execute!(io::stderr(), MoveToColumn(new_col))?;
            self.cursor_position.set_col(new_col);
        }
        execute!(io::stderr(), SavePosition)?;
        self.write_bottom()?;
        execute!(io::stderr(), RestorePosition)?;
        
        return Ok(());
    }
    
    pub fn move_cursor_right(&mut self) -> io::Result<()> {
        let (col, row) = self.cursor_position.get();
        let (_, len, _) = self.buffer[row as usize];
        let max_index = len;
        if col + 1 >= max_index as u16 {
            return Ok(());
        }
        execute!(io::stderr(), MoveRight(1), SavePosition)?;
        self.stored_position.set_col(col + 1);
        self.cursor_position.move_right();
        self.write_bottom()?;
        execute!(io::stderr(), RestorePosition)?; 
        return Ok(());
    }
    
    pub fn select_entry(&mut self) -> io::Result<()> {
        let (path, _, _) = &self.buffer[(self.index + self.stored_position.row()) as usize];
        
        if path.is_none() {
            return Ok(());
        }
        
        let path = path.as_ref().unwrap().as_path();
    
        if path.is_dir() {
            self.cd = path.to_path_buf();
            self.generate_buffer();
            execute!(io::stderr(), 
                Clear(ClearType::All), 
                MoveTo(0,0)
            )?;
            self.cursor_position = Position(START_X, START_Y);
            self.draw_screen()?;
            execute!(io::stderr(), MoveTo(START_X, START_Y))?;
            self.stored_position = Position(START_X, START_Y);
            self.index = 0;
        } else {
            if path.ends_with(".desktop") {
    
                return Ok(());
            }
    
            if path.is_executable() {
                let path = path.absolutize().unwrap().display().to_string();
                std::process::Command::new(&path)
                    .stdin(Stdio::piped())
                    .stderr(Stdio::piped())
                    .stdout(Stdio::piped())
                    .spawn()
                    .expect(&format!("could not spawn process {}", &path));
            } else {
                opener::open(path).map_err(|_e| {
                    return io::Error::new(
                        ErrorKind::Other,
                        _e.to_string()
                    )
                })?;
            }
        }
    
        return Ok(());
    }
    
    pub fn window_resize(&mut self, width: u16, height: u16) -> io::Result<()> {
        let resized_width = max(width, 5);
        let resized_height = max(height, 5);
    
        if width < 5 || height < 5 {
            execute!(io::stderr(), SetSize(resized_width, resized_height))?;
        }
    
        let (col, row) = self.cursor_position.get();
        
        self.stored_position.set_row(min(row, resized_height - BOTTOM_RESERVED - 1));
    
        let (_, len, _) = self.buffer[(self.index + self.stored_position.row()) as usize];
        let max_col = max(len - 1, 0) as u16;
    
        if max_col < col {
            execute!(io::stderr(), MoveTo(max_col, self.stored_position.row()))?;
        } else if col < min(self.stored_position.col(), max_col) {
            execute!(io::stderr(), MoveTo(min(self.stored_position.col(), max_col), self.stored_position.row()))?;
        }
    
        execute!(io::stderr(), SavePosition)?;
        self.draw_screen()?;
        execute!(io::stderr(), RestorePosition)?;
    
        return Ok(());
    }
}

impl Default for App {
    fn default() -> Self {
        Self { 
            buffer: vec![], 
            cd: std::env::current_dir().unwrap(), 
            output: String::new(), 
            index: 0, 
            stored_position: Position(START_X, START_Y), 
            cursor_position: Position(START_X, START_Y), 
            command_state: CommandState::default() 
        }
    }
}

impl CommandState {
    fn push(&mut self, digit: u32) {
        self.number = Some(match self.number {
            Some(n) => n * 10 + digit as u16,
            None => digit as u16
        });
    }
}

impl Default for CommandState {
    fn default() -> Self {
        Self { 
            number: None, 
            prefix: Prefix::None 
        }
    }
}

impl Display for CommandState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut output: String = String::new();
        if let Some(num) = self.number {
            output += &num.to_string();
        }

        write!(f, "{}{}{empty: <10}", output, self.prefix, empty = "")
    }
}

impl Display for Prefix {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let output = match self {
            Prefix::f => "f",
            Prefix::F => "F",
            Prefix::g => "g",
            Prefix::t => "t",
            Prefix::T => "T",
            Prefix::z => "z",
            _ => ""
        };
        write!(f, "{}", output)
    }
}

pub trait StoreEmpty {
    fn append_empty(&mut self);
}

impl StoreEmpty for Vec<(Option<PathBuf>, usize, String)> {
    fn append_empty(&mut self) {
        self.push((None, 1, " ".to_string()));
    }
}
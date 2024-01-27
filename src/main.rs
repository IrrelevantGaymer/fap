use std::io::{self, Write};

use app::App;
use crossterm::{
    cursor::MoveTo,
    terminal::{
        disable_raw_mode, enable_raw_mode, 
        DisableLineWrap, EnableLineWrap, 
        EnterAlternateScreen, LeaveAlternateScreen, 
        SetTitle
    },
    execute
};

mod app;
mod color_config;
mod panic_guard;

use panic_guard::GuardWithHook;

use crate::app::LEFT_SIDE_PADDING;

const BOTTOM_RESERVED: u16 = 3;
const START_X: u16 = 0;
const START_Y: u16 = 3;

fn main() -> io::Result<()> {
    let mut app = App::default();
    app.output = app.cd.display().to_string();

    execute!(io::stderr(), 
        EnterAlternateScreen, 
        SetTitle("fap"), 
        MoveTo(0, 0), 
        DisableLineWrap
    )?;
    {
        let _guard = GuardWithHook::new(|| 
            execute!(io::stderr(), LeaveAlternateScreen, EnableLineWrap).unwrap()
        );

        enable_raw_mode()?;
        {
            let _guard = GuardWithHook::new(|| disable_raw_mode().unwrap());

            io::stderr().flush()?;
            app.generate_buffer();
            app.draw_screen()?;
            execute!(io::stderr(), MoveTo(LEFT_SIDE_PADDING + START_X, START_Y))?;
            app.read_input()?;
        }
    }

    println!("{}", app.cd.display());
    return Ok(());
}

fn write_to_screen(msg: String) -> io::Result<()> {
    write!(io::stderr(), "{}", msg)?;
    return Ok(());
}

fn writeln_to_screen(msg: String) -> io::Result<()> {
    write!(io::stderr(), "{}\n\r", msg)?;
    return Ok(());
}

pub struct Position {
    pub x: u16,
    pub y: u16
}

impl Position {
    pub fn new(x: u16, y: u16) -> Position {
        Position {x, y}
    }
    
    pub fn get(&self) -> (u16, u16) {
        (self.x, self.y)
    }

    pub fn move_left(&mut self) {
        self.x -= 1;
    }

    pub fn move_up(&mut self) {
        self.y -= 1;
    }

    pub fn move_down(&mut self) {
        self.y += 1;
    }

    pub fn move_right(&mut self) {
        self.x += 1;
    }

    pub fn set_col(&mut self, col: u16) {
        self.x = col;
    }

    pub fn set_row(&mut self, row: u16) {
        self.y = row;
    }

    pub fn col(&self) -> u16 {
        self.x
    }

    pub fn row(&self) -> u16 {
        self.y
    }
}
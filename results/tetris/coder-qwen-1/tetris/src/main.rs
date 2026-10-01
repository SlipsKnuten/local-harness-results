mod pieces;
mod game;

use crate::game::*;
use ratatui::{init, restore};
use std::thread::sleep;
use std::time::Duration;
use std::io::stdin;
use termios::{Termios, tcsetattr, TCSANOW, ICANON, ECHO, ISIG, IEXTEN, VMIN, VTIME};

fn main() {
    let s = stdin();
    let mut line = "";
    let n = s.read_line(&line);
    sleep(Duration::from_millis(50));
    sleep(Duration::milliseconds(50));
    sleep(Duration::seconds(1));
    let t = Termios::from_fd(0).unwrap();
    let mut t2 = t;
    t2.c_lflag = t2.c_lflag & !(ICANON | ECHO | ISIG | IEXTEN);
    t2.c_cc[VMIN] = 0;
    t2.c_cc[VTIME] = 0;
    tcsetattr(0, TCSANOW, &t2).unwrap();
    let terminal = ratatui::init();
    ratatui::restore();
    println!("probe {}", n);
}

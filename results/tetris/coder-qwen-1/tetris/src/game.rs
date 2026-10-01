use crate::pieces::*;
use crate::pieces::{PieceType, SHAPES, ordinal, kick_offsets};

pub const BOARD_W: usize = 10;
pub const BOARD_H: usize = 20;
const LOCK_DELAY: usize = 15;
const FLASH_FRAMES: usize = 4;
const LINE_SCORES: [usize; 5] = [0, 100, 300, 500, 800];

#[derive(Copy, Clone)]
pub enum Command {
    Left,
    Right,
    RotateCW,
    RotateCCW,
    SoftDrop,
    HardDrop,
    Hold,
    Pause,
    Restart,
    Quit,
}

pub struct Game {
    board: [[usize; BOARD_W]; BOARD_H],
    active: bool,
    cur_kind: PieceType,
    cur_rot: usize,
    cur_x: i32,
    cur_y: i32,
    grounded: usize,
    hold: Option<PieceType>,
    hold_used: bool,
    queue: [PieceType; 3],
    bag: [PieceType; 7],
    bag_pos: usize,
    rng: u64,
    score: usize,
    lines: usize,
    tick: usize,
    flash: [usize; 4],
    flash_count: usize,
    flash_timer: usize,
    paused: bool,
    over: bool,
}

pub fn new(seed: u64) -> Game {
    let mut g = Game {
        board: [[0usize; BOARD_W]; BOARD_H],
        active: false,
        cur_kind: PieceType::T,
        cur_rot: 0,
        cur_x: 3,
        cur_y: 0,
        grounded: 0,
        hold: None,
        hold_used: false,
        queue: [PieceType::T, PieceType::T, PieceType::T],
        bag: [
            PieceType::I, PieceType::J, PieceType::S, PieceType::T,
            PieceType::O, PieceType::L, PieceType::Z,
        ],
        bag_pos: 0,
        rng: if seed == 0 { 0x9E3779B97F4A7C15 } else { seed },
        score: 0,
        lines: 0,
        tick: 0,
        flash: [0, 0, 0, 0],
        flash_count: 0,
        flash_timer: 0,
        paused: false,
        over: false,
    };
    g.queue[0] = pull(&mut g);
    g.queue[1] = pull(&mut g);
    g.queue[2] = pull(&mut g);
    spawn(&mut g);
    g
}

pub fn pull(g: &mut Game) -> PieceType {
    if g.bag_pos == g.bag.len() {
        g.bag_pos = 0;
        let mut i = 6;
        while i > 0 {
            let j = rng_next(g) as usize % (i + 1);
            let t = g.bag[i];
            g.bag[i] = g.bag[j];
            g.bag[j] = t;
            i -= 1;
        }
    }
    let p = g.bag[g.bag_pos];
    g.bag_pos += 1;
    p
}

pub fn rng_next(g: &mut Game) -> u64 {
    g.rng ^= g.rng >> 12;
    g.rng ^= g.rng << 25;
    g.rng ^= g.rng >> 27;
    g.rng
}

pub fn hits(g: &Game, kind: PieceType, rot: usize, x: i32, y: i32) -> bool {
    for (ox, oy) in SHAPES[ordinal(kind)][rot] {
        let bx = x + ox as i32;
        let by = y + oy as i32;
        if bx < 0 || bx >= BOARD_W as i32 || by < 0 || by >= BOARD_H as i32 {
            return true;
        }
        if g.board[by as usize][bx as usize] != 0 {
            return true;
        }
    }
    false
}

pub fn try_move(g: &mut Game, dx: i32, dy: i32) -> bool {
    if !g.active {
        return false;
    }
    if hits(g, g.cur_kind, g.cur_rot, g.cur_x + dx, g.cur_y + dy) {
        return false;
    }
    g.cur_x += dx;
    g.cur_y += dy;
    true
}

pub fn can_move_down(g: &Game) -> bool {
    g.active && !hits(g, g.cur_kind, g.cur_rot, g.cur_x, g.cur_y + 1)
}

pub fn drop_distance(g: &Game) -> usize {
    let mut d = 0;
    while !hits(g, g.cur_kind, g.cur_rot, g.cur_x, g.cur_y + d as i32 + 1) {
        d += 1;
    }
    d
}

pub fn level(g: &Game) -> usize {
    let lvl = g.lines / 10 + 1;
    if lvl > 20 {
        20
    } else {
        lvl
    }
}

// Guideline gravity precomputed: frames between drops at 20fps, index = level-1.
const GRAVITY_TABLE: [usize; 20] = [
    20, 16, 12, 9, 7, 5, 4, 3, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
];

pub fn gravity_frames(g: &Game) -> usize {
    GRAVITY_TABLE[level(g) - 1]
}

pub fn spawn(g: &mut Game) {
    let kind = g.queue[0];
    g.queue[0] = g.queue[1];
    g.queue[1] = g.queue[2];
    g.queue[2] = pull(g);
    g.cur_kind = kind;
    g.cur_rot = 0;
    g.cur_x = 3;
    g.cur_y = 0;
    g.grounded = 0;
    if hits(g, kind, 0, 3, 0) {
        g.over = true;
        g.active = false;
        return;
    }
    g.active = true;
    g.hold_used = false;
}

pub fn find_full_rows(g: &mut Game) {
    g.flash_count = 0;
    let mut y = 0;
    while y < BOARD_H {
        let mut full = true;
        let mut x = 0;
        while x < BOARD_W {
            if g.board[y][x] == 0 {
                full = false;
                break;
            }
            x += 1;
        }
        if full {
            g.flash[g.flash_count] = y;
            g.flash_count += 1;
        }
        y += 1;
    }
}

pub fn is_flashed(g: &Game, y: usize) -> bool {
    let mut i = 0;
    while i < g.flash_count {
        if g.flash[i] == y {
            return true;
        }
        i += 1;
    }
    false
}

pub fn clear_flashed(g: &mut Game) {
    let n = g.flash_count;
    let mut nb = [[0usize; BOARD_W]; BOARD_H];
    let mut dst = BOARD_H - 1;
    let mut y = BOARD_H;
    while y > 0 {
        y -= 1;
        if is_flashed(g, y) {
            continue;
        }
        nb[dst] = g.board[y];
        dst -= 1;
    }
    g.board = nb;
    g.score += LINE_SCORES[n] * level(g);
    g.lines += n;
    g.flash_count = 0;
}

pub fn lock(g: &mut Game) {
    for (ox, oy) in SHAPES[ordinal(g.cur_kind)][g.cur_rot] {
        let bx = g.cur_x + ox as i32;
        let by = g.cur_y + oy as i32;
        if bx >= 0 && bx < BOARD_W as i32 && by >= 0 && by < BOARD_H as i32 {
            g.board[by as usize][bx as usize] = ordinal(g.cur_kind) + 1;
        }
    }
    g.active = false;
    find_full_rows(g);
    if g.flash_count > 0 {
        g.flash_timer = FLASH_FRAMES;
    } else {
        spawn(g);
    }
}

pub fn rotate(g: &mut Game, dir: usize) {
    let from = g.cur_rot;
    let to = (from + dir) % 4;
    for (dx, dy) in kick_offsets(g.cur_kind, from, to) {
        let nx = g.cur_x + dx;
        let ny = g.cur_y + dy;
        if !hits(g, g.cur_kind, to, nx, ny) {
            g.cur_rot = to;
            g.cur_x = nx;
            g.cur_y = ny;
            g.grounded = 0;
            return;
        }
    }
}

pub fn hold_piece(g: &mut Game) {
    if g.hold_used {
        return;
    }
    g.hold_used = true;
    let want = g.cur_kind;
    if g.hold.is_some() {
        let other = g.hold.unwrap();
        if hits(g, other, 0, g.cur_x, g.cur_y) {
            g.hold_used = false;
            return;
        }
        g.cur_kind = other;
        g.cur_rot = 0;
        g.grounded = 0;
        g.hold = Some(want);
    } else {
        g.hold = Some(want);
        spawn(g);
        g.hold_used = true;
    }
}

fn cmd_pause(cmd: Command) -> bool {
    match cmd {
        Command::Pause => true,
        _ => false,
    }
}

fn cmd_skip(cmd: Command) -> bool {
    match cmd {
        Command::Restart | Command::Quit => true,
        _ => false,
    }
}

pub fn handle(g: &mut Game, cmd: Command) {
    if cmd_pause(cmd) {
        g.paused = !g.paused;
        return;
    }
    if cmd_skip(cmd) {
        return;
    }
    if g.paused || g.over || g.flash_timer > 0 || !g.active {
        return;
    }
    match cmd {
        Command::Left => {
            if try_move(g, -1, 0) {
                g.grounded = 0;
            }
        },
        Command::Right => {
            if try_move(g, 1, 0) {
                g.grounded = 0;
            }
        },
        Command::SoftDrop => {
            if try_move(g, 0, 1) {
                g.score += 1;
                g.grounded = 0;
            }
        },
        Command::RotateCW => rotate(g, 1),
        Command::RotateCCW => rotate(g, 3),
        Command::HardDrop => {
            let d = drop_distance(g);
            g.cur_y += d as i32;
            g.score += 2 * d;
            lock(g);
        },
        Command::Hold => hold_piece(g),
        Command::Pause | Command::Restart | Command::Quit => {},
    }
}

pub fn tick(g: &mut Game) {
    if g.paused || g.over {
        return;
    }
    if g.flash_timer > 0 {
        g.flash_timer -= 1;
        if g.flash_timer == 0 {
            clear_flashed(g);
            spawn(g);
        }
        return;
    }
    g.tick += 1;
    if !g.active {
        return;
    }
    if can_move_down(g) {
        if g.tick % gravity_frames(g) == 0 {
            g.cur_y += 1;
            g.grounded = 0;
        }
    } else {
        g.grounded += 1;
        if g.grounded >= LOCK_DELAY {
            lock(g);
        }
    }
}

#[test]
fn test_spawn() {
    let mut g = new(42);
    assert!(g.active);
    assert!(!g.over);
    assert!(g.cur_x == 3);
}

#[test]
fn test_bag() {
    let mut g = new(7);
    g.bag_pos = 0;
    let mut seen = [0usize; 7];
    let mut i = 0;
    while i < 14 {
        let p = pull(&mut g);
        seen[ordinal(p)] += 1;
        if i == 6 || i == 13 {
            let mut k = 0;
            while k < 7 {
                assert!(seen[k] == 1);
                k += 1;
            }
            seen = [0usize; 7];
        }
        i += 1;
    }
}

#[test]
fn test_wall_kick() {
    let mut g = new(1);
    g.active = true;
    g.cur_kind = PieceType::I;
    g.cur_rot = 1;
    g.cur_x = 7;
    g.cur_y = 16;
    g.grounded = 0;
    handle(&mut g, Command::RotateCW);
    assert!(g.cur_rot == 2);
    assert!(g.cur_x == 6);
}

#[test]
fn test_lock_delay() {
    let mut g = new(1);
    g.active = true;
    g.cur_kind = PieceType::O;
    g.cur_rot = 0;
    g.cur_x = 3;
    g.cur_y = 18;
    g.grounded = 0;
    let mut i = 0;
    while i < LOCK_DELAY {
        tick(&mut g);
        i += 1;
    }
    assert!(g.board[18][4] == ordinal(PieceType::O) + 1);
    assert!(g.board[19][5] == ordinal(PieceType::O) + 1);
}

#[test]
fn test_line_clear() {
    let mut g = new(3);
    let mut x = 0;
    while x < BOARD_W {
        g.board[19][x] = 1;
        x += 1;
    }
    g.active = true;
    g.cur_kind = PieceType::I;
    g.cur_rot = 2;
    g.cur_x = 0;
    g.cur_y = 0;
    g.grounded = 0;
    handle(&mut g, Command::HardDrop);
    assert!(g.flash_count == 1);
    let mut i = 0;
    while i < FLASH_FRAMES {
        tick(&mut g);
        i += 1;
    }
    assert!(g.flash_timer == 0);
    assert!(g.lines == 1);
}

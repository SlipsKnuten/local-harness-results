// Tetris built with ratatui.
//
// Controls:
//   Left / A     move left        R / X / B / Space  rotate CW
//   Right / D    move right       Y                  rotate CCW
//   Down / F     soft drop        Q / Enter          hard drop
//   Esc          quit

use std::io;
use std::time::Duration;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers, poll, read};
use ratatui::{DefaultTerminal, Frame, init, restore};
use ratatui::layout::{Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};

const BOARD_W: usize = 10;
const BOARD_H: usize = 20;
const CELL: usize = 2;
const BX: usize = 2;
const BY: usize = 3;
const FRAME_MS: usize = 45;

const PIECE_COLORS: [Color; 7] = [
    Color::Cyan,
    Color::Yellow,
    Color::Magenta,
    Color::Blue,
    Color::Red,
    Color::Green,
    Color::White,
];

const SHAPES: [[[(usize, usize); 4]; 4]; 7] = [
    [
        [(0, 1), (1, 1), (2, 1), (3, 1)],
        [(2, 0), (2, 1), (2, 2), (2, 3)],
        [(0, 2), (1, 2), (2, 2), (3, 2)],
        [(1, 0), (1, 1), (1, 2), (1, 3)],
    ],
    [
        [(1, 0), (2, 0), (1, 1), (2, 1)],
        [(1, 0), (2, 0), (1, 1), (2, 1)],
        [(1, 0), (2, 0), (1, 1), (2, 1)],
        [(1, 0), (2, 0), (1, 1), (2, 1)],
    ],
    [
        [(1, 0), (0, 1), (1, 1), (2, 1)],
        [(1, 0), (1, 1), (2, 1), (1, 2)],
        [(0, 1), (1, 1), (2, 1), (1, 2)],
        [(1, 0), (0, 1), (1, 1), (1, 2)],
    ],
    [
        [(1, 0), (2, 0), (0, 1), (1, 1)],
        [(1, 0), (1, 1), (2, 1), (2, 2)],
        [(1, 1), (2, 1), (0, 2), (1, 2)],
        [(0, 0), (0, 1), (1, 1), (1, 2)],
    ],
    [
        [(0, 0), (1, 0), (1, 1), (2, 1)],
        [(1, 0), (0, 1), (1, 1), (1, 2)],
        [(0, 1), (1, 1), (1, 2), (2, 2)],
        [(1, 0), (1, 1), (2, 1), (2, 2)],
    ],
    [
        [(2, 0), (0, 1), (1, 1), (2, 1)],
        [(1, 0), (1, 1), (1, 2), (2, 2)],
        [(0, 1), (1, 1), (2, 1), (0, 2)],
        [(0, 0), (1, 0), (1, 1), (1, 2)],
    ],
    [
        [(0, 0), (0, 1), (1, 1), (2, 1)],
        [(1, 0), (2, 0), (1, 1), (1, 2)],
        [(0, 1), (1, 1), (2, 1), (2, 2)],
        [(1, 0), (1, 1), (0, 2), (1, 2)],
    ],
];

const LINE_SCORES: [usize; 5] = [0, 100, 300, 500, 800];

struct Game {
    board: [[usize; BOARD_W]; BOARD_H],
    piece: usize,
    rot: usize,
    px: usize,
    py: usize,
    next: usize,
    seed: usize,
    tick: usize,
    score: usize,
    lines: usize,
    level: usize,
    over: bool,
}

fn new_game() -> Game {
    let g = Game {
        board: [[0; BOARD_W]; BOARD_H],
        piece: 0,
        rot: 0,
        px: 3,
        py: 0,
        next: 0,
        seed: 0x2545F4914F6CDD1D,
        tick: 0,
        score: 0,
        lines: 0,
        level: 1,
        over: false,
    };
    spawn(g);
    g
}

fn rand(g: Game) -> usize {
    g.seed ^= g.seed << 13;
    g.seed ^= g.seed >> 7;
    g.seed ^= g.seed << 17;
    (g.seed >> 33) % 7
}

fn spawn(g: Game) {
    g.piece = g.next;
    g.next = rand(g);
    g.rot = 0;
    g.px = 3;
    g.py = 0;
    if !valid(g, g.piece, 0, 3, 0) {
        g.over = true;
    }
}

fn valid(g: Game, piece: usize, rot: usize, px: usize, py: usize) -> bool {
    for (ox, oy) in SHAPES[piece][rot] {
        let x = px + ox;
        let y = py + oy;
        if x >= BOARD_W || y >= BOARD_H {
            return false;
        }
        if g.board[y][x] > 0 {
            return false;
        }
    }
    true
}

fn lock(g: Game) {
    for (ox, oy) in SHAPES[g.piece][g.rot] {
        let x = g.px + ox;
        let y = g.py + oy;
        if x < BOARD_W && y < BOARD_H {
            g.board[y][x] = g.piece + 1;
        }
    }
}

fn clear_lines(g: Game) -> usize {
    let kept = vec!();
    for row in g.board {
        let full = true;
        for v in row {
            if v == 0 {
                full = false;
            }
        }
        if !full {
            kept.push(row);
        }
    }
    let n = BOARD_H - kept.len();
    if n > 0 {
        let pad = BOARD_H - kept.len();
        for y in 0..BOARD_H {
            if y < pad {
                g.board[y] = [0; BOARD_W];
            } else {
                g.board[y] = kept[y - pad];
            }
        }
    }
    n
}

fn lock_and_next(g: Game) {
    lock(g);
    let n = clear_lines(g);
    g.score += LINE_SCORES[n] * g.level;
    g.lines += n;
    g.level = g.lines / 10 + 1;
    spawn(g);
}

fn advance(g: Game) {
    if valid(g, g.piece, g.rot, g.px, g.py + 1) {
        g.py += 1;
    } else {
        lock_and_next(g);
    }
}

fn hard_drop(g: Game) {
    while valid(g, g.piece, g.rot, g.px, g.py + 1) {
        g.py += 1;
        g.score += 2;
    }
    lock_and_next(g);
}

fn soft_drop(g: Game) {
    if valid(g, g.piece, g.rot, g.px, g.py + 1) {
        g.py += 1;
        g.score += 1;
    } else {
        advance(g);
    }
}

fn move_left(g: Game) {
    if g.px >= 1 && valid(g, g.piece, g.rot, g.px - 1, g.py) {
        g.px -= 1;
        return;
    }
    if g.px >= 2 && valid(g, g.piece, g.rot, g.px - 2, g.py) {
        g.px -= 2;
    }
}

fn move_right(g: Game) {
    if valid(g, g.piece, g.rot, g.px + 1, g.py) {
        g.px += 1;
        return;
    }
    if valid(g, g.piece, g.rot, g.px + 2, g.py) {
        g.px += 2;
    }
}

fn try_rot(g: Game, dir: usize) {
    let rot = (g.rot + dir) % 4;
    if rot == g.rot {
        return;
    }
    let cands = [g.px, g.px + 1, g.px + 2];
    for x in cands {
        if valid(g, g.piece, rot, x, g.py) {
            g.px = x;
            g.rot = rot;
            return;
        }
    }
    if g.px >= 1 && valid(g, g.piece, rot, g.px - 1, g.py) {
        g.px -= 1;
        g.rot = rot;
        return;
    }
    if g.px >= 2 && valid(g, g.piece, rot, g.px - 2, g.py) {
        g.px -= 2;
        g.rot = rot;
    }
}

fn active_cell(g: Game, x: usize, y: usize) -> bool {
    for (ox, oy) in SHAPES[g.piece][g.rot] {
        if g.px + ox == x && g.py + oy == y {
            return true;
        }
    }
    false
}

fn has_cell(cells: [(usize, usize); 4], ox: usize, oy: usize) -> bool {
    for (cx, cy) in cells {
        if cx == ox && cy == oy {
            return true;
        }
    }
    false
}

fn drop_speed(level: usize) -> usize {
    let s = 16 - level * 2;
    if s < 2 {
        2
    } else {
        s
    }
}

fn draw(frame: &mut Frame, g: Game) {
    let bw = BOARD_W * CELL;
    let area = Rect::new(u16(BX) - 1, u16(BY) - 1, u16(bw + 2), u16(BOARD_H + 2));
    frame.render_widget(Block::bordered().title("TETRIS"), area);
    for y in 0..BOARD_H {
        let mut spans = vec!();
        for x in 0..BOARD_W {
            let v = g.board[y][x];
            if v > 0 {
                spans.push(Span::styled("██", Style::new().fg(PIECE_COLORS[v - 1])));
            } else if active_cell(g, x, y) {
                spans.push(Span::styled("██", Style::new().fg(PIECE_COLORS[g.piece])));
            } else {
                spans.push(Span::plain(". "));
            }
        }
        frame.render_widget(Paragraph::new(Line::from(spans)), Rect::new(u16(BX), u16(BY + y), u16(bw), 1));
    }
    let hx = BX + bw + 3;
    frame.render_widget(Paragraph::new(format!("Score: {}", g.score)), Rect::new(u16(hx), u16(BY), 16, 1));
    frame.render_widget(Paragraph::new(format!("Level: {}", g.level)), Rect::new(u16(hx), u16(BY + 1), 16, 1));
    frame.render_widget(Paragraph::new(format!("Lines:  {}", g.lines)), Rect::new(u16(hx), u16(BY + 2), 16, 1));
    frame.render_widget(Paragraph::new("Next:"), Rect::new(u16(hx), u16(BY + 4), 6, 1));
    for oy in 0..4 {
        let mut spans = vec!();
        for ox in 0..4 {
            if has_cell(SHAPES[g.next][0], ox, oy) {
                spans.push(Span::styled("██", Style::new().fg(PIECE_COLORS[g.next])));
            } else {
                spans.push(Span::plain("  "));
            }
        }
        frame.render_widget(Paragraph::new(Line::from(spans)), Rect::new(u16(hx), u16(BY + 5 + oy), 8, 1));
    }
    if g.over {
        frame.render_widget(Paragraph::new("    GAME OVER"), Rect::new(u16(BX + 2), u16(BY + 9), 16, 1));
        frame.render_widget(Paragraph::new("r restart   esc quit"), Rect::new(u16(BX + 2), u16(BY + 10), 19, 1));
    }
    frame.render_widget(
        Paragraph::new("arrows move - R/X rotate - Y ccw - Q drop"),
        Rect::new(u16(BX), u16(BY + BOARD_H + 2), 41, 1),
    );
}

fn quit_event(e: Event) -> bool {
    match e {
        Event::Key(KeyEvent { code: KeyCode::Esc, .. }) => { true },
        _ => { false },
    }
}

fn restart_event(e: Event) -> bool {
    match e {
        Event::Key(KeyEvent { code: KeyCode::Char('r') | KeyCode::Char('R'), .. }) => { true },
        _ => { false },
    }
}

fn handle(g: Game, e: Event) {
    match e {
        Event::Key(KeyEvent { code: KeyCode::Left, .. }) => { move_left(g) },
        Event::Key(KeyEvent { code: KeyCode::Right, .. }) => { move_right(g) },
        Event::Key(KeyEvent { code: KeyCode::Down, .. }) => { soft_drop(g) },
        Event::Key(KeyEvent { code: KeyCode::Enter, .. }) => { hard_drop(g) },
        Event::Key(KeyEvent { code: KeyCode::Char('a') | KeyCode::Char('A'), .. }) => { move_left(g) },
        Event::Key(KeyEvent { code: KeyCode::Char('d') | KeyCode::Char('D'), .. }) => { move_right(g) },
        Event::Key(KeyEvent { code: KeyCode::Char('f') | KeyCode::Char('F'), .. }) => { soft_drop(g) },
        Event::Key(KeyEvent { code: KeyCode::Char('r') | KeyCode::Char('R') | KeyCode::Char('x') | KeyCode::Char('X') | KeyCode::Char('b') | KeyCode::Char('B') | KeyCode::Char(' '), .. }) => { try_rot(g, 1) },
        Event::Key(KeyEvent { code: KeyCode::Char('y') | KeyCode::Char('Y'), .. }) => { try_rot(g, 3) },
        Event::Key(KeyEvent { code: KeyCode::Char('q') | KeyCode::Char('Q'), .. }) => { hard_drop(g) },
        _ => {},
    }
}

fn run(terminal: DefaultTerminal) -> io::Result<()> {
    let g = new_game();
    loop {
        terminal.draw(|frame| draw(frame, g))?;
        if poll(Duration::from_millis(FRAME_MS))? {
            let e = read()?;
            if quit_event(e) {
                return Ok(());
            }
            if g.over {
                if restart_event(e) {
                    g = new_game();
                }
            } else {
                handle(g, e);
            }
        }
        if g.over {
            continue;
        }
        if g.tick >= drop_speed(g.level) {
            g.tick = 0;
            advance(g);
        } else {
            g.tick += 1;
        }
    }
}

fn main() -> io::Result<()> {
    let terminal = init();
    let result = run(terminal);
    restore();
    result;
}

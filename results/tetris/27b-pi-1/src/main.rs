mod board;
mod game;
mod piece;

use board::{Cell, HEIGHT, WIDTH};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use game::Game;
use piece::PieceType;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame, Terminal,
};
use std::io::{self, Stdout};
use std::time::{Duration, Instant};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run(&mut terminal);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    result
}

fn run(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<(), Box<dyn std::error::Error>> {
    let mut game = Game::new();
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| game.draw(f))?;

        // Wait for input, but wake up when the next gravity tick is due so the
        // piece keeps falling smoothly even without any keypresses.
        let elapsed = last_tick.elapsed();
        let tick = Duration::from_millis(game.tick_ms());
        let timeout = if elapsed >= tick {
            Duration::ZERO
        } else {
            tick - elapsed
        };

        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                        KeyCode::Char('p') => game.toggle_pause(),
                        KeyCode::Char('r') if game.game_over => game.restart(),
                        KeyCode::Left | KeyCode::Char('a') => game.move_left(),
                        KeyCode::Right | KeyCode::Char('d') => game.move_right(),
                        KeyCode::Down | KeyCode::Char('s') => game.soft_drop(),
                        KeyCode::Up | KeyCode::Char('w') | KeyCode::Char('x') => game.rotate(1),
                        KeyCode::Char('z') => game.rotate(-1),
                        KeyCode::Char(' ') => game.hard_drop(),
                        _ => {}
                    }
                }
            }
        }

        if last_tick.elapsed() >= tick {
            game.tick();
            last_tick = Instant::now();
        }
    }
}

impl Game {
    pub fn draw(&self, f: &mut Frame) {
        let area = f.area();
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length((WIDTH * 2) as u16 + 2),
                Constraint::Min(18),
            ])
            .split(area);

        self.draw_board(f, chunks[0]);
        self.draw_sidebar(f, chunks[1]);

        if self.game_over || self.paused {
            let msg = if self.game_over { "GAME OVER" } else { "PAUSED" };
            self.draw_overlay(f, area, msg);
        }
    }

    fn draw_board(&self, f: &mut Frame, area: Rect) {
        // Combine locked cells and the active piece into a display grid.
        let mut display: Vec<Vec<Option<PieceType>>> = (0..HEIGHT)
            .map(|y| {
                (0..WIDTH)
                    .map(|x| match self.board.grid[y][x] {
                        Cell::Filled(pt) => Some(pt),
                        Cell::Empty => None,
                    })
                    .collect()
            })
            .collect();
        for &(dx, dy) in self.current.blocks() {
            let x = self.current.x + dx;
            let y = self.current.y + dy;
            if (0..WIDTH as i32).contains(&x) && (0..HEIGHT as i32).contains(&y) {
                display[y as usize][x as usize] = Some(self.current.shape);
            }
        }

        let lines: Vec<Line> = display
            .iter()
            .map(|row| {
                let spans: Vec<Span> = row
                    .iter()
                    .map(|&cell| match cell {
                        Some(pt) => Span::styled(
                            "██".to_string(),
                            Style::default().fg(pt.color()).add_modifier(Modifier::BOLD),
                        ),
                        None => Span::raw("  "),
                    })
                    .collect();
                Line::from(spans)
            })
            .collect();

        let board = Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title(" TETRIS "));
        f.render_widget(board, area);
    }

    fn draw_sidebar(&self, f: &mut Frame, area: Rect) {
        let mut lines: Vec<Line> = Vec::new();

        lines.push(Line::from(Span::styled(
            "NEXT",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(" ".to_string()));
        for row in self.next_preview() {
            lines.push(row);
        }
        lines.push(Line::from(" ".to_string()));

        lines.push(Line::from(Span::styled(
            format!("Score: {}", self.score),
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(format!("Lines: {}", self.lines)));
        lines.push(Line::from(format!("Level: {}", self.level)));
        lines.push(Line::from(" ".to_string()));

        for (key, action) in [
            ("←/a", "move left"),
            ("→/d", "move right"),
            ("↓/s", "soft drop"),
            ("↑/x", "rotate cw"),
            ("z", "rotate ccw"),
            ("space", "hard drop"),
            ("p", "pause"),
            ("q", "quit"),
        ] {
            lines.push(Line::from(vec![
                Span::styled(format!("{key:<5}"), Style::default().fg(Color::Cyan)),
                Span::raw(action.to_string()),
            ]));
        }

        let panel = Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" INFO "),
        );
        f.render_widget(panel, area);
    }

    /// Render the next piece as a small 4x4 grid.
    fn next_preview(&self) -> Vec<Line<'_>> {
        let preview = piece::Piece::new(self.next);
        let mut cells = [[false; 4]; 4];
        for &(dx, dy) in preview.blocks() {
            let x = dx as usize;
            let y = dy as usize;
            if x < 4 && y < 4 {
                cells[y][x] = true;
            }
        }
        (0..4)
            .map(|y| {
                let spans: Vec<Span> = (0..4)
                    .map(|x| {
                        if cells[y][x] {
                            Span::styled(
                                "██".to_string(),
                                Style::default()
                                    .fg(self.next.color())
                                    .add_modifier(Modifier::BOLD),
                            )
                        } else {
                            Span::raw("  ")
                        }
                    })
                    .collect();
                Line::from(spans)
            })
            .collect()
    }

    fn draw_overlay(&self, f: &mut Frame, area: Rect, msg: &str) {
        let popup = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(area.height / 3),
                Constraint::Length(5),
                Constraint::Min(0),
            ])
            .split(area)[1];

        let hint = if self.game_over {
            "Press 'r' to restart, 'q' to quit"
        } else {
            "Press 'p' to resume"
        };
        let text = vec![
            Line::from(Span::styled(
                msg,
                Style::default()
                    .fg(Color::Red)
                    .add_modifier(Modifier::BOLD | Modifier::REVERSED),
            )),
            Line::from(""),
            Line::from(Span::styled(
                hint,
                Style::default().fg(Color::White),
            )),
        ];
        let width = msg.len() as u16 + 8;
        let block = Block::default().borders(Borders::ALL).title(" ").style(
            Style::default().bg(Color::Black),
        );
        let para = Paragraph::new(text)
            .style(Style::default().bg(Color::Black))
            .block(block)
            .centered();

        let centered = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length((popup.width / 2).saturating_sub(width / 2)),
                Constraint::Length(width),
                Constraint::Min(0),
            ])
            .split(popup)[1];

        f.render_widget(para, centered);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::Board;
    use crate::piece::Piece;
    use ratatui::backend::TestBackend;

    fn count_filled(board: &Board) -> usize {
        board
            .grid
            .iter()
            .flatten()
            .filter(|c| matches!(c, Cell::Filled(_)))
            .count()
    }

    #[test]
    fn renders_board_and_piece() {
        let backend = TestBackend::new(60, 30);
        let mut term = Terminal::new(backend).unwrap();
        let game = Game::new();
        term.draw(|f| game.draw(f)).unwrap();
        let buf = term.backend().buffer();
        let content: String = buf.content().iter().map(|c| c.symbol()).collect();
        assert!(content.contains("TETRIS"), "board title missing");
        assert!(content.contains('█'), "no piece blocks rendered");
        assert!(content.contains("Score:"));
        assert!(content.contains("NEXT"));
    }

    #[test]
    fn hard_drop_locks_piece_and_spawns_next() {
        let mut game = Game::new();
        let before = count_filled(&game.board);
        assert_eq!(before, 0);
        game.hard_drop();
        let after = count_filled(&game.board);
        assert_eq!(after, 4, "a tetromino should add exactly 4 locked blocks");
        assert!(!game.game_over);
    }

    #[test]
    fn move_and_rotate_respect_walls() {
        let mut game = Game::new();
        // Walk the piece fully to the left wall; x must never go below 0.
        for _ in 0..20 {
            game.move_left();
        }
        assert!(game.current.x >= 0);
        // Rotating against a wall must never make it overlap the board.
        for _ in 0..8 {
            game.rotate(1);
        }
        assert!(!game.board.collides(&game.current));
    }

    #[test]
    fn clear_lines_scores_and_levels_up() {
        let mut board = Board::new();
        for x in 0..WIDTH {
            board.grid[HEIGHT - 1][x] = Cell::Filled(PieceType::I);
        }
        // A lone block just above the full row; it should fall into the bottom.
        board.grid[HEIGHT - 2][0] = Cell::Filled(PieceType::T);
        let cleared = board.clear_lines();
        assert_eq!(cleared, 1);
        assert_eq!(board.grid[HEIGHT - 1][0], Cell::Filled(PieceType::T));
    }

    #[test]
    fn rotations_are_consistent() {
        for shape in PieceType::ALL {
            let base = Piece::new(shape);
            for r in 0..4 {
                let mut p = base;
                p.rot = r;
                let blocks: Vec<(i32, i32)> = p.blocks().to_vec();
                assert_eq!(blocks.len(), 4, "{shape:?} has wrong block count");
                let mut unique = blocks.clone();
                unique.sort();
                unique.dedup();
                assert_eq!(unique.len(), 4, "{shape:?} rotation {r} has duplicates");
            }
            // A full 360° spin returns to the starting state.
            let start = base.blocks().to_vec();
            let mut p = base;
            p.rot = 4 % 4;
            assert_eq!(p.blocks().to_vec(), start);
        }
        // The O piece is symmetric: every rotation looks identical.
        let o = Piece::new(PieceType::O);
        let r0 = o.blocks().to_vec();
        for r in 1..4 {
            let mut p = o;
            p.rot = r;
            assert_eq!(p.blocks().to_vec(), r0, "O rotated {} differs", r);
        }
    }

    #[test]
    fn game_over_when_stacked_to_top() {
        let mut game = Game::new();
        // A fully-filled column in the spawn area blocks any new piece without
        // completing a single row (so no line clears to hide the top-out).
        for y in 0..HEIGHT {
            game.board.grid[y][3] = Cell::Filled(PieceType::I);
        }
        game.hard_drop(); // locks + respawns into the blocked column
        assert!(game.game_over);
    }

    #[test]
    fn pause_freezes_movement() {
        let mut game = Game::new();
        game.toggle_pause();
        assert!(game.paused);
        let x = game.current.x;
        game.move_right();
        assert_eq!(game.current.x, x, "movement ignored while paused");
        game.tick();
        game.toggle_pause();
    }
}

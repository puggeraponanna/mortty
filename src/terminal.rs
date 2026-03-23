use vte::{Params, Perform};

#[derive(Clone, Default)]
pub struct Cell {
    pub c: char,
}

pub struct Terminal {
    pub grid: Vec<Vec<Cell>>,
    pub cursor_col: usize,
    pub cursor_row: usize,
    pub cols: usize,
    pub rows: usize,
}

impl Terminal {
    pub fn new(cols: usize, rows: usize) -> Self {
        let grid = vec![vec![Cell::default(); cols]; rows];
        Self {
            grid,
            cursor_col: 0,
            cursor_row: 0,
            cols,
            rows,
        }
    }
}

impl Perform for Terminal {
    fn print(&mut self, c: char) {
        if self.cursor_col < self.cols && self.cursor_row < self.rows {
            self.grid[self.cursor_row][self.cursor_col].c = c;
            self.cursor_col += 1;
        }
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            10 | 11 | 12 => { // LF, VT, FF (Scroll)
                if self.cursor_row == self.rows - 1 {
                    for r in 0..self.rows - 1 {
                        self.grid[r] = self.grid[r + 1].clone();
                    }
                    for c in 0..self.cols {
                        self.grid[self.rows - 1][c].c = '\0';
                    }
                } else {
                    self.cursor_row += 1;
                }
            }
            13 => { // \r CR
                self.cursor_col = 0;
            }
            8 => { // Backspace
                if self.cursor_col > 0 {
                    self.cursor_col -= 1;
                }
            }
            _ => {
                // Ignore other control characters for now
            }
        }
    }

    fn hook(&mut self, _params: &Params, _intermediates: &[u8], _ignore: bool, _action: char) {}
    fn put(&mut self, _byte: u8) {}
    fn unhook(&mut self) {}
    fn osc_dispatch(&mut self, _params: &[&[u8]], _bell_terminated: bool) {}
    fn csi_dispatch(&mut self, params: &Params, _intermediates: &[u8], _ignore: bool, action: char) {
        match action {
            'J' => {
                let mut it = params.iter();
                let param = it.next().map(|p| p[0]).unwrap_or(0);
                match param {
                    0 => {
                        for c in self.cursor_col..self.cols {
                            self.grid[self.cursor_row][c].c = '\0';
                        }
                        for r in self.cursor_row + 1..self.rows {
                            for c in 0..self.cols {
                                self.grid[r][c].c = '\0';
                            }
                        }
                    }
                    2 | 3 => {
                        for r in 0..self.rows {
                            for c in 0..self.cols {
                                self.grid[r][c].c = '\0';
                            }
                        }
                        if param == 2 {
                            self.cursor_col = 0;
                            self.cursor_row = 0;
                        }
                    }
                    _ => {}
                }
            }
            'K' => {
                let mut it = params.iter();
                let param = it.next().map(|p| p[0]).unwrap_or(0);
                match param {
                    0 => {
                        for c in self.cursor_col..self.cols {
                            self.grid[self.cursor_row][c].c = '\0';
                        }
                    }
                    2 => {
                        for c in 0..self.cols {
                            self.grid[self.cursor_row][c].c = '\0';
                        }
                    }
                    _ => {}
                }
            }
            'H' | 'f' => {
                let mut it = params.iter();
                let row = it.next().map(|p| p[0]).unwrap_or(1) as usize;
                let col = it.next().map(|p| p[0]).unwrap_or(1) as usize;
                self.cursor_row = row.saturating_sub(1).min(self.rows - 1);
                self.cursor_col = col.saturating_sub(1).min(self.cols - 1);
            }
            'A' => {
                let mut it = params.iter();
                let param = it.next().map(|p| p[0]).unwrap_or(1) as usize;
                self.cursor_row = self.cursor_row.saturating_sub(param);
            }
            'B' => {
                let mut it = params.iter();
                let param = it.next().map(|p| p[0]).unwrap_or(1) as usize;
                self.cursor_row = (self.cursor_row + param).min(self.rows - 1);
            }
            'C' => {
                let mut it = params.iter();
                let param = it.next().map(|p| p[0]).unwrap_or(1) as usize;
                self.cursor_col = (self.cursor_col + param).min(self.cols - 1);
            }
            'D' => {
                let mut it = params.iter();
                let param = it.next().map(|p| p[0]).unwrap_or(1) as usize;
                self.cursor_col = self.cursor_col.saturating_sub(param);
            }
            'G' | '`' => {
                let mut it = params.iter();
                let param = it.next().map(|p| p[0]).unwrap_or(1) as usize;
                self.cursor_col = param.saturating_sub(1).min(self.cols - 1);
            }
            'P' => { // Delete Character
                let mut it = params.iter();
                let param = it.next().map(|p| p[0]).unwrap_or(1) as usize;
                let delete_count = param.max(1);
                let row = self.cursor_row;
                let col = self.cursor_col;
                for c in col..self.cols {
                    if c + delete_count < self.cols {
                        self.grid[row][c].c = self.grid[row][c + delete_count].c;
                    } else {
                        self.grid[row][c].c = '\0';
                    }
                }
            }
            '@' => { // Insert Character
                let mut it = params.iter();
                let param = it.next().map(|p| p[0]).unwrap_or(1) as usize;
                let insert_count = param.max(1);
                let row = self.cursor_row;
                let col = self.cursor_col;
                for c in (col..self.cols).rev() {
                    if c >= col + insert_count {
                        self.grid[row][c].c = self.grid[row][c - insert_count].c;
                    } else {
                        self.grid[row][c].c = '\0';
                    }
                }
            }
            'X' => { // Erase Character
                let mut it = params.iter();
                let param = it.next().map(|p| p[0]).unwrap_or(1) as usize;
                let erase_count = param.max(1);
                let row = self.cursor_row;
                let col = self.cursor_col;
                for c in col..col + erase_count {
                    if c < self.cols {
                        self.grid[row][c].c = '\0';
                    }
                }
            }
            'm' => {
                // SGR: Colors and styles
            }
            'h' | 'l' => {
                // Set/Reset Mode (e.g., cursor visibility, bracketed paste)
            }
            'r' | 's' | 'u' => {
                // Scrolling Region / Save & Restore Cursor
            }
            _ => {
                println!("Unhandled CSI action: {}", action);
            }
        }
    }
    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, _byte: u8) {}
}

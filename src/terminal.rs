use vte::{Params, Perform};

/// Convert an xterm 256-color index to (r, g, b).
fn ansi_256_to_rgb(n: u8) -> [u8; 3] {
    match n {
        // Standard 16 colors
        0  => [12, 12, 12],
        1  => [205, 49, 49],
        2  => [13, 188, 121],
        3  => [229, 229, 16],
        4  => [36, 114, 200],
        5  => [188, 63, 188],
        6  => [17, 168, 205],
        7  => [229, 229, 229],
        8  => [102, 102, 102],
        9  => [241, 76, 76],
        10 => [35, 209, 139],
        11 => [245, 245, 67],
        12 => [59, 142, 234],
        13 => [214, 112, 214],
        14 => [41, 184, 219],
        15 => [255, 255, 255],
        // 6×6×6 color cube: indices 16–231
        16..=231 => {
            let idx = n - 16;
            let b = idx % 6;
            let g = (idx / 6) % 6;
            let r = idx / 36;
            let f = |x: u8| if x == 0 { 0 } else { 55 + x * 40 };
            [f(r), f(g), f(b)]
        }
        // Grayscale ramp: indices 232–255
        232..=255 => {
            let v = 8 + (n - 232) as u16 * 10;
            let v = v as u8;
            [v, v, v]
        }
    }
}

#[derive(Clone)]
pub struct Cell {
    pub c: char,
    pub fg: [u8; 3],
    pub bg: [u8; 3],
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            c: '\0',
            fg: [200, 200, 200],
            bg: [12, 12, 12],
        }
    }
}

pub struct Terminal {
    pub grid: Vec<Vec<Cell>>,
    pub cursor_col: usize,
    pub cursor_row: usize,
    pub cols: usize,
    pub rows: usize,
    pub current_fg: [u8; 3],
    pub current_bg: [u8; 3],
    pub dirty: bool,
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
            current_fg: [200, 200, 200],
            current_bg: [12, 12, 12],
            dirty: true,
        }
    }

    pub fn clear_dirty(&mut self) {
        self.dirty = false;
    }

    pub fn resize(&mut self, new_cols: usize, new_rows: usize) {
        let mut new_grid = vec![vec![Cell::default(); new_cols]; new_rows];
        for r in 0..self.rows.min(new_rows) {
            for c in 0..self.cols.min(new_cols) {
                new_grid[r][c] = self.grid[r][c].clone();
            }
        }
        self.grid = new_grid;
        self.cols = new_cols;
        self.rows = new_rows;
        self.cursor_col = self.cursor_col.min(new_cols.saturating_sub(1));
        self.cursor_row = self.cursor_row.min(new_rows.saturating_sub(1));
        self.dirty = true;
    }
}

impl Perform for Terminal {
    fn print(&mut self, c: char) {
        if self.cursor_col < self.cols && self.cursor_row < self.rows {
            self.grid[self.cursor_row][self.cursor_col].c = c;
            self.grid[self.cursor_row][self.cursor_col].fg = self.current_fg;
            self.grid[self.cursor_row][self.cursor_col].bg = self.current_bg;
            self.cursor_col += 1;
            self.dirty = true;
        }
    }

    fn execute(&mut self, byte: u8) {
        self.dirty = true;
        match byte {
            10 | 11 | 12 => { // LF, VT, FF (Scroll)
                if self.cursor_row == self.rows - 1 {
                    for r in 0..self.rows - 1 {
                        self.grid[r] = self.grid[r + 1].clone();
                    }
                    for c in 0..self.cols {
                        self.grid[self.rows - 1][c] = Cell::default();
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
        self.dirty = true;
        match action {
            'J' => {
                let mut it = params.iter();
                let param = it.next().map(|p| p[0]).unwrap_or(0);
                match param {
                    0 => {
                        for c in self.cursor_col..self.cols {
                            self.grid[self.cursor_row][c] = Cell::default();
                        }
                        for r in self.cursor_row + 1..self.rows {
                            for c in 0..self.cols {
                                self.grid[r][c] = Cell::default();
                            }
                        }
                    }
                    2 | 3 => {
                        for r in 0..self.rows {
                            for c in 0..self.cols {
                                self.grid[r][c] = Cell::default();
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
                            self.grid[self.cursor_row][c] = Cell::default();
                        }
                    }
                    1 => {
                        for c in 0..=self.cursor_col {
                            self.grid[self.cursor_row][c] = Cell::default();
                        }
                    }
                    2 => {
                        for c in 0..self.cols {
                            self.grid[self.cursor_row][c] = Cell::default();
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
                        self.grid[row][c] = Cell::default();
                    }
                }
            }
            'm' => {
                let mut fg = self.current_fg;
                let mut bg = self.current_bg;

                if params.is_empty() {
                    fg = [200, 200, 200];
                    bg = [12, 12, 12];
                }

                let all_params: Vec<u16> = params.iter().flat_map(|p| p.iter().copied()).collect();
                let mut i = 0;
                while i < all_params.len() {
                    let param = all_params[i];
                    match param {
                        0 => { fg = [200, 200, 200]; bg = [12, 12, 12]; i += 1; }
                        30 => { fg = [0, 0, 0]; i += 1; }
                        31 => { fg = [205, 49, 49]; i += 1; }
                        32 => { fg = [13, 188, 121]; i += 1; }
                        33 => { fg = [229, 229, 16]; i += 1; }
                        34 => { fg = [36, 114, 200]; i += 1; }
                        35 => { fg = [188, 63, 188]; i += 1; }
                        36 => { fg = [17, 168, 205]; i += 1; }
                        37 => { fg = [229, 229, 229]; i += 1; }
                        38 => {
                            if i + 2 < all_params.len() && all_params[i+1] == 5 {
                                // 256-color fg [38;5;N]
                                fg = ansi_256_to_rgb(all_params[i+2] as u8);
                                i += 3;
                            } else if i + 4 < all_params.len() && all_params[i+1] == 2 {
                                // true-color [38;2;R;G;B]
                                fg = [all_params[i+2] as u8, all_params[i+3] as u8, all_params[i+4] as u8];
                                i += 5;
                            } else {
                                i += 1;
                            }
                        }
                        39 => { fg = [200, 200, 200]; i += 1; }
                        40 => { bg = [0, 0, 0]; i += 1; }
                        41 => { bg = [205, 49, 49]; i += 1; }
                        42 => { bg = [13, 188, 121]; i += 1; }
                        43 => { bg = [229, 229, 16]; i += 1; }
                        44 => { bg = [36, 114, 200]; i += 1; }
                        45 => { bg = [188, 63, 188]; i += 1; }
                        46 => { bg = [17, 168, 205]; i += 1; }
                        47 => { bg = [229, 229, 229]; i += 1; }
                        48 => {
                            if i + 2 < all_params.len() && all_params[i+1] == 5 {
                                // 256-color bg [48;5;N]
                                bg = ansi_256_to_rgb(all_params[i+2] as u8);
                                i += 3;
                            } else if i + 4 < all_params.len() && all_params[i+1] == 2 {
                                // true-color [48;2;R;G;B]
                                bg = [all_params[i+2] as u8, all_params[i+3] as u8, all_params[i+4] as u8];
                                i += 5;
                            } else {
                                i += 1;
                            }
                        }
                        49 => { bg = [12, 12, 12]; i += 1; }
                        90 => { fg = [102, 102, 102]; i += 1; }
                        91 => { fg = [241, 76, 76]; i += 1; }
                        92 => { fg = [35, 209, 139]; i += 1; }
                        93 => { fg = [245, 245, 67]; i += 1; }
                        94 => { fg = [59, 142, 234]; i += 1; }
                        95 => { fg = [214, 112, 214]; i += 1; }
                        96 => { fg = [41, 184, 219]; i += 1; }
                        97 => { fg = [229, 229, 229]; i += 1; }
                        _ => { i += 1; }
                    }
                }
                self.current_fg = fg;
                self.current_bg = bg;
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

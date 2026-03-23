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
            10 => { // \n LF
                self.cursor_row = (self.cursor_row + 1).min(self.rows - 1);
            }
            13 => { // \r CR
                self.cursor_col = 0;
            }
            8 => { // Backspace
                if self.cursor_col > 0 {
                    self.cursor_col -= 1;
                }
            }
            _ => {}
        }
    }

    fn hook(&mut self, _params: &Params, _intermediates: &[u8], _ignore: bool, _action: char) {}
    fn put(&mut self, _byte: u8) {}
    fn unhook(&mut self) {}
    fn osc_dispatch(&mut self, _params: &[&[u8]], _bell_terminated: bool) {}
    fn csi_dispatch(&mut self, _params: &Params, _intermediates: &[u8], _ignore: bool, _action: char) {}
    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, _byte: u8) {}
}

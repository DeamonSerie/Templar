use set_core::GridSize;

pub fn index_to_coord(grid: &GridSize, idx: u64) -> (u32, u32) {
    let w = grid.w as u64;
    let row = (idx / w) as u32;
    let col = (idx % w) as u32;
    (row, col)
}

pub fn coord_to_index(grid: &GridSize, row: u32, col: u32) -> u64 {
    row as u64 * grid.w as u64 + col as u64
}

pub fn clamp_price(grid: &GridSize, idx: i64) -> u64 {
    let cells = grid.cells() as i64;
    if cells == 0 {
        return 0;
    }
    let mut v = idx % cells;
    if v < 0 {
        v += cells;
    }
    v as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use set_core::GridSize;
    #[test]
    fn roundtrip() {
        let g = GridSize { w: 4, h: 5 };
        assert_eq!(index_to_coord(&g, 7), (1, 3));
        assert_eq!(coord_to_index(&g, 1, 3), 7);
    }
    #[test]
    fn clamp() {
        let g = GridSize { w: 4, h: 5 };
        assert_eq!(clamp_price(&g, -1), 19);
        assert_eq!(clamp_price(&g, 20), 0);
    }
}

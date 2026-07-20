//! Flow packing for the widget grid: given each widget's cell footprint and a
//! column count, assign every widget a `(col, row)` grid position by scanning
//! left-to-right, top-to-bottom for the first free slot that fits. Order is
//! preserved (Apple-style: you order widgets, you don't pin coordinates), so
//! resizing the window only changes the column count and reflows. Pure and
//! unit-testable without egui.

/// Place `footprints` (each `(width, height)` in cells) into `columns` columns.
/// Returns one `(col, row)` per input, in the same order. A footprint wider
/// than the grid is clamped to the column count.
pub fn pack(footprints: &[(u8, u8)], columns: usize) -> Vec<(usize, usize)> {
    let columns = columns.max(1);
    // Occupancy grid; rows are appended lazily as widgets need them.
    let mut occ: Vec<Vec<bool>> = Vec::new();
    let mut out = Vec::with_capacity(footprints.len());

    for &(w, h) in footprints {
        let w = (w as usize).clamp(1, columns);
        let h = (h as usize).max(1);
        let (col, row) = first_free_slot(&occ, columns, w, h);

        // Grow the grid to cover the placed footprint.
        while occ.len() < row + h {
            occ.push(vec![false; columns]);
        }
        for r in row..row + h {
            for c in col..col + w {
                occ[r][c] = true;
            }
        }
        out.push((col, row));
    }

    out
}

/// The number of grid rows a packing occupies — the max `row + height` over
/// all placements. Zero when empty. (The renderer derives grid height from the
/// packed rects directly; this remains for the packing unit tests.)
#[cfg_attr(not(test), allow(dead_code))]
pub fn row_count(placements: &[(usize, usize)], footprints: &[(u8, u8)]) -> usize {
    placements
        .iter()
        .zip(footprints)
        .map(|(&(_, row), &(_, h))| row + h as usize)
        .max()
        .unwrap_or(0)
}

fn first_free_slot(occ: &[Vec<bool>], columns: usize, w: usize, h: usize) -> (usize, usize) {
    let mut row = 0;
    loop {
        for col in 0..=columns - w {
            if fits(occ, col, row, w, h) {
                return (col, row);
            }
        }
        row += 1;
    }
}

fn fits(occ: &[Vec<bool>], col: usize, row: usize, w: usize, h: usize) -> bool {
    for r in row..row + h {
        // Rows beyond the current occupancy are entirely free.
        if r >= occ.len() {
            continue;
        }
        for c in col..col + w {
            if occ[r][c] {
                return false;
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packs_singles_left_to_right_then_wraps() {
        let items = [(1, 1); 5];
        let placed = pack(&items, 2);
        assert_eq!(placed, vec![(0, 0), (1, 0), (0, 1), (1, 1), (0, 2)]);
        assert_eq!(row_count(&placed, &items), 3);
    }

    #[test]
    fn wide_widget_wraps_to_next_row_when_it_cannot_fit() {
        // 1x1, then a 2x1 that can't share row 0 in a 2-col grid.
        let items = [(1, 1), (2, 1)];
        let placed = pack(&items, 2);
        assert_eq!(placed, vec![(0, 0), (0, 1)]);
    }

    #[test]
    fn small_widget_backfills_gap_left_by_a_tall_one() {
        // Large (2x2) at origin, then two smalls fill the column beside it is
        // impossible (no room), so verify a taller layout backfills correctly:
        // small, then large — small takes (0,0), large needs a 2-wide row so
        // goes to row 1, leaving (1,0) free for a third small to backfill.
        let items = [(1, 1), (2, 2), (1, 1)];
        let placed = pack(&items, 2);
        assert_eq!(placed[0], (0, 0));
        assert_eq!(placed[1], (0, 1));
        assert_eq!(placed[2], (1, 0)); // backfilled the gap next to the first
        assert_eq!(row_count(&placed, &items), 3);
    }

    #[test]
    fn footprint_wider_than_grid_is_clamped() {
        let items = [(4, 1)];
        let placed = pack(&items, 2);
        assert_eq!(placed, vec![(0, 0)]);
    }

    #[test]
    fn empty_input_has_no_rows() {
        let placed = pack(&[], 3);
        assert!(placed.is_empty());
        assert_eq!(row_count(&placed, &[]), 0);
    }
}

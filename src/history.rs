use std::collections::VecDeque;

/// Fixed-capacity rolling window of samples. Pushing past capacity drops the
/// oldest sample — an O(1) ring buffer, unlike `Vec` + `remove(0)`.
pub struct History {
    buf: VecDeque<f64>,
    cap: usize,
}

impl History {
    /// A history pre-filled with `cap` zeros, so charts have a full window
    /// from the first frame.
    pub fn new(cap: usize) -> Self {
        let mut buf = VecDeque::with_capacity(cap);
        buf.extend(std::iter::repeat(0.0).take(cap));
        Self { buf, cap }
    }

    pub fn push(&mut self, value: f64) {
        if self.buf.len() == self.cap {
            self.buf.pop_front();
        }
        self.buf.push_back(value);
    }

    pub fn iter(&self) -> impl Iterator<Item = f64> + '_ {
        self.buf.iter().copied()
    }

    pub fn last(&self) -> f64 {
        self.buf.back().copied().unwrap_or(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_full_of_zeros() {
        let h = History::new(3);
        assert_eq!(h.iter().collect::<Vec<_>>(), vec![0.0, 0.0, 0.0]);
        assert_eq!(h.last(), 0.0);
    }

    #[test]
    fn push_evicts_oldest_and_keeps_capacity() {
        let mut h = History::new(3);
        h.push(1.0);
        h.push(2.0);
        h.push(3.0);
        // window is now [1,2,3]
        assert_eq!(h.iter().collect::<Vec<_>>(), vec![1.0, 2.0, 3.0]);

        h.push(4.0); // evicts the 1.0
        assert_eq!(h.iter().collect::<Vec<_>>(), vec![2.0, 3.0, 4.0]);
        assert_eq!(h.last(), 4.0);
    }

    #[test]
    fn last_reflects_most_recent_push() {
        let mut h = History::new(2);
        h.push(42.0);
        assert_eq!(h.last(), 42.0);
    }
}

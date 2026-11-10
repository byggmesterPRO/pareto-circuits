//! Which designs are worth having, and which one fits a wish.
//!
//! A design has a size and a delay. Its size is its gates, or, when it has to be balanced, its gates plus the
//! timers that balance it. The frontier is every design that no other design beats on both: going down it,
//! each one is slower than the one before and smaller. Any wish of the kind "the smallest that is at most
//! this slow" or "the fastest that is at most this big" is answered by a design on the frontier.
//!
//! Two designs with the same gates and ticks are told apart by what they cost balanced: the one that needs
//! fewer parts then is the better circuit to have.

/// A measured design, as far as choosing goes.
#[derive(Clone, Copy, Debug)]
pub struct Point {
    pub gates: u64,
    pub ticks: u64,
    /// Gates plus timers once it is balanced at the same ticks. None if it cannot be (a circuit with memory).
    pub balanced: Option<u64>,
}

/// What is asked for.
#[derive(Clone, Copy, Debug, Default)]
pub struct Spec {
    /// Every path the same number of ticks, so the inputs can change every tick.
    pub balanced: bool,
    pub max_ticks: Option<u64>,
    /// Counts the timers too when balanced.
    pub max_gates: Option<u64>,
    /// Pick the fastest that fits. Without it, the smallest.
    pub fastest: bool,
}

impl Point {
    /// The size that counts: gates, or gates and timers when it has to be balanced.
    pub fn size(&self, balanced: bool) -> Option<u64> {
        if balanced { self.balanced } else { Some(self.gates) }
    }

    /// Smaller is better between designs of the same size and ticks.
    fn tie(&self) -> (u64, u64) {
        (self.balanced.unwrap_or(u64::MAX), self.gates)
    }
}

/// The designs no other design beats on both size and ticks, fastest first. Of designs with the same size and
/// ticks only the best one is kept (see `Point::tie`), and of those the first.
pub fn frontier(points: &[Point], balanced: bool) -> Vec<usize> {
    let mut all: Vec<(u64, u64, (u64, u64), usize)> = points.iter().enumerate().filter_map(|(i, p)| Some((p.ticks, p.size(balanced)?, p.tie(), i))).collect();
    all.sort();
    let mut out: Vec<usize> = vec![];
    let mut smallest = u64::MAX;
    for (_, size, _, i) in all {
        if size < smallest {
            smallest = size;
            out.push(i);
        }
    }
    out
}

/// The design that fits the wish best, if any does.
pub fn pick(points: &[Point], spec: &Spec) -> Option<usize> {
    points
        .iter()
        .enumerate()
        .filter_map(|(i, p)| Some((p.size(spec.balanced)?, p, i)))
        .filter(|(size, p, _)| spec.max_ticks.is_none_or(|t| p.ticks <= t) && spec.max_gates.is_none_or(|g| *size <= g))
        .min_by_key(|(size, p, i)| if spec.fastest { (p.ticks, *size, p.tie(), *i) } else { (*size, p.ticks, p.tie(), *i) })
        .map(|(_, _, i)| i)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(gates: u64, ticks: u64, balanced: u64) -> Point {
        Point { gates, ticks, balanced: Some(balanced) }
    }

    #[test]
    fn the_frontier_keeps_what_nothing_beats() {
        // 0 fast and big, 1 beaten by 0, 2 in between, 3 small and slow, 4 the same as 3 but worse balanced
        let all = [p(100, 4, 110), p(120, 5, 121), p(60, 6, 90), p(20, 30, 50), p(20, 30, 70)];
        assert_eq!(frontier(&all, false), [0, 2, 3]);
        // balanced, the small one costs more than it looks, but it is still the smallest
        assert_eq!(frontier(&all, true), [0, 2, 3]);
        let all = [p(100, 4, 110), p(20, 30, 400)];
        assert_eq!(frontier(&all, false), [0, 1]);
        assert_eq!(frontier(&all, true), [0]);
    }

    #[test]
    fn equal_designs_go_to_the_one_that_balances_cheaper() {
        let all = [p(50, 8, 90), p(50, 8, 60), p(50, 8, 60)];
        assert_eq!(frontier(&all, false), [1]);
        assert_eq!(pick(&all, &Spec::default()), Some(1));
    }

    #[test]
    fn a_pick_is_the_smallest_that_fits() {
        let all = [p(100, 4, 110), p(60, 6, 90), p(20, 30, 50)];
        let spec = |balanced, max_ticks, max_gates, fastest| Spec { balanced, max_ticks, max_gates, fastest };
        assert_eq!(pick(&all, &spec(false, None, None, false)), Some(2));
        assert_eq!(pick(&all, &spec(false, Some(8), None, false)), Some(1));
        assert_eq!(pick(&all, &spec(false, Some(5), None, false)), Some(0));
        assert_eq!(pick(&all, &spec(false, Some(3), None, false)), None);
        assert_eq!(pick(&all, &spec(false, None, None, true)), Some(0));
        assert_eq!(pick(&all, &spec(false, None, Some(70), true)), Some(1));
        // balanced, sizes are 110, 90 and 50
        assert_eq!(pick(&all, &spec(true, None, Some(70), true)), Some(2));
        assert_eq!(pick(&all, &spec(true, Some(8), Some(89), false)), None);
        // a circuit with memory has no balanced form
        assert_eq!(pick(&[Point { gates: 9, ticks: 3, balanced: None }], &spec(true, None, None, false)), None);
    }
}

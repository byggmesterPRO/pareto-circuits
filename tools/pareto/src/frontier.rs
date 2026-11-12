//! Which designs are worth having, and which one fits a wish.
//!
//! A design is measured on several axes, and smaller is better on each. Without memory there are two: its
//! size and its ticks. Its size is its gates, or, when it has to be balanced, its gates plus the timers that
//! balance it. With memory one number for the delay is not enough, so there are five: from an input to the
//! outputs (read), from a pulse to the outputs (write), how long the inputs have to be steady before a pulse
//! (setup), how often operations can come (period), and the gates.
//!
//! The frontier is every design that no other design matches or beats on all of its axes. Without memory
//! those are only ticks and size, so a wish like "the smallest that is at most this slow" is answered by a
//! design on it, but one with `max_period` may be met only by a design off it. So `pick` looks at every
//! design, not only the frontier.
//!
//! Two designs that are the same on every axis are told apart by what they cost balanced: the one that needs
//! fewer parts then is the better circuit to have.

use logic_sim::verify::Timing;

/// A measured design, as far as choosing goes.
#[derive(Clone, Copy, Debug)]
pub struct Point {
    pub gates: u64,
    /// The one delay a build's meta.json gives.
    pub ticks: u64,
    /// Gates plus timers once it is balanced at the same ticks. None if it cannot be (a circuit with memory).
    pub balanced: Option<u64>,
    pub timing: Timing,
}

/// What is asked for. Every `max` is "at most this many", and one that is None asks for nothing.
#[derive(Clone, Copy, Debug, Default)]
pub struct Wish {
    /// Every path the same number of ticks, so the inputs can change every tick. Without memory only.
    pub balanced: bool,
    pub max_ticks: Option<u64>,
    /// Counts the timers too when balanced.
    pub max_gates: Option<u64>,
    pub max_read: Option<u64>,
    /// These two are for a family with memory; a design without has no such delay and never fits them.
    pub max_write: Option<u64>,
    pub max_setup: Option<u64>,
    /// How often new inputs, or operations, can come.
    pub max_period: Option<u64>,
    /// Pick the fastest that fits. Without it, the smallest.
    pub fastest: bool,
}

impl Point {
    /// The size that counts: gates, or gates and timers when it has to be balanced.
    pub fn size(&self, balanced: bool) -> Option<u64> {
        if balanced { self.balanced } else { Some(self.gates) }
    }

    /// How often new inputs can come: every tick once balanced.
    pub fn period(&self, balanced: bool) -> Option<u64> {
        if balanced { self.balanced.map(|_| 1) } else { self.timing.period }
    }

    /// What it is measured on, fastest things first and the size last. None if it has no balanced form and
    /// that is what is asked.
    pub fn axes(&self, balanced: bool) -> Option<Vec<u64>> {
        let size = self.size(balanced)?;
        Some(match self.timing {
            Timing { write: Some(write), setup, period, read, .. } => vec![read, write, setup.unwrap_or(0), period.unwrap_or(u64::MAX), size],
            // not the period: a design off the frontier can have a shorter one, which is why `pick` looks at all
            _ => vec![self.ticks, size],
        })
    }

    /// Smaller is better between designs that are the same on every axis.
    fn tie(&self) -> (u64, u64) {
        (self.balanced.unwrap_or(u64::MAX), self.gates)
    }

    fn fits(&self, wish: &Wish) -> bool {
        let within = |have: Option<u64>, max: Option<u64>| max.is_none_or(|max| have.is_some_and(|have| have <= max));
        within(Some(self.ticks), wish.max_ticks)
            && within(self.size(wish.balanced), wish.max_gates)
            && within(Some(self.timing.read), wish.max_read)
            && within(self.timing.write, wish.max_write)
            && within(self.timing.setup, wish.max_setup)
            && within(self.period(wish.balanced), wish.max_period)
    }
}

/// The designs no other design matches or beats on every axis, in the order of their axes (so the fastest
/// first). Of designs that are the same on every axis only the best one is kept (see `Point::tie`), and of
/// those the first.
pub fn frontier(points: &[Point], balanced: bool) -> Vec<usize> {
    let all: Vec<(Vec<u64>, (u64, u64), usize)> = points.iter().enumerate().filter_map(|(i, p)| Some((p.axes(balanced)?, p.tie(), i))).collect();
    let beaten = |me: &(Vec<u64>, (u64, u64), usize)| {
        all.iter().any(|other| {
            let no_worse = other.0.iter().zip(&me.0).all(|(o, m)| o <= m);
            no_worse && (other.0 != me.0 || (other.1, other.2) < (me.1, me.2))
        })
    };
    let mut out: Vec<&(Vec<u64>, (u64, u64), usize)> = all.iter().filter(|me| !beaten(me)).collect();
    out.sort();
    out.into_iter().map(|p| p.2).collect()
}

/// The design that fits the wish best, if any does.
pub fn pick(points: &[Point], wish: &Wish) -> Option<usize> {
    points
        .iter()
        .enumerate()
        .filter(|(_, p)| p.fits(wish))
        .filter_map(|(i, p)| Some((p.size(wish.balanced)?, p.axes(wish.balanced)?, p, i)))
        .min_by_key(|(size, axes, p, i)| if wish.fastest { (p.ticks, axes.clone(), *size, p.tie(), *i) } else { (*size, axes.clone(), p.ticks, p.tie(), *i) })
        .map(|(_, _, _, i)| i)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A design without memory: gates, ticks, its size balanced, and how often its inputs can change as it is.
    fn p(gates: u64, ticks: u64, balanced: u64) -> Point {
        Point { gates, ticks, balanced: Some(balanced), timing: Timing { read: ticks, write: None, setup: None, period: Some(ticks - 1), pulse_at: None } }
    }

    /// A design with memory.
    fn m(gates: u64, read: u64, write: u64, setup: u64, period: u64) -> Point {
        Point { gates, ticks: write, balanced: None, timing: Timing { read, write: Some(write), setup: Some(setup), period: Some(period), pulse_at: Some(setup) } }
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
        assert_eq!(pick(&all, &Wish::default()), Some(1));
    }

    #[test]
    fn a_pick_is_the_smallest_that_fits() {
        let all = [p(100, 4, 110), p(60, 6, 90), p(20, 30, 50)];
        let wish = |balanced, max_ticks, max_gates, fastest| Wish { balanced, max_ticks, max_gates, fastest, ..Wish::default() };
        assert_eq!(pick(&all, &wish(false, None, None, false)), Some(2));
        assert_eq!(pick(&all, &wish(false, Some(8), None, false)), Some(1));
        assert_eq!(pick(&all, &wish(false, Some(5), None, false)), Some(0));
        assert_eq!(pick(&all, &wish(false, Some(3), None, false)), None);
        assert_eq!(pick(&all, &wish(false, None, None, true)), Some(0));
        assert_eq!(pick(&all, &wish(false, None, Some(70), true)), Some(1));
        // balanced, sizes are 110, 90 and 50
        assert_eq!(pick(&all, &wish(true, None, Some(70), true)), Some(2));
        assert_eq!(pick(&all, &wish(true, Some(8), Some(89), false)), None);
        // as they are, their inputs can change every 3, 5 and 29 ticks; balanced, every tick
        assert_eq!(pick(&all, &Wish { max_period: Some(5), ..Wish::default() }), Some(1));
        assert_eq!(pick(&all, &Wish { max_period: Some(1), ..Wish::default() }), None);
        assert_eq!(pick(&all, &Wish { max_period: Some(1), balanced: true, ..Wish::default() }), Some(2));
        // a design without memory has no pulse to wait for
        assert_eq!(pick(&all, &Wish { max_write: Some(99), ..Wish::default() }), None);
    }

    #[test]
    fn with_memory_every_delay_is_an_axis() {
        // 0 reads fast, 1 writes fast, 2 is small, 3 is 2 with a longer period, 4 is beaten by 0 on everything
        let all = [m(200, 4, 5, 5, 9), m(210, 5, 3, 5, 9), m(90, 9, 9, 9, 18), m(90, 9, 9, 9, 20), m(220, 4, 6, 5, 9)];
        assert_eq!(frontier(&all, false), [0, 1, 2]);
        assert!(frontier(&all, true).is_empty());
        assert_eq!(pick(&all, &Wish::default()), Some(2));
        assert_eq!(pick(&all, &Wish { max_read: Some(4), ..Wish::default() }), Some(0));
        assert_eq!(pick(&all, &Wish { max_write: Some(4), ..Wish::default() }), Some(1));
        assert_eq!(pick(&all, &Wish { max_period: Some(9), max_setup: Some(5), ..Wish::default() }), Some(0));
        assert_eq!(pick(&all, &Wish { max_period: Some(8), ..Wish::default() }), None);
        assert_eq!(pick(&all, &Wish { balanced: true, ..Wish::default() }), None);
        // one whose period was not found fits no wish for one
        let lost = Point { timing: Timing { period: None, ..all[2].timing }, ..all[2] };
        assert_eq!(pick(&[lost], &Wish { max_period: Some(99), ..Wish::default() }), None);
        assert_eq!(pick(&[lost], &Wish::default()), Some(0));
    }
}

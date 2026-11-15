//! Assignment of threads to the entries of a fixed palette (CONVERSION §3.4).
//!
//! Two strategies:
//!
//! - [`MatchStrategy::Nearest`]: each thread gets the entry with the smallest
//!   ΔE00. Fast, but two close threads may get the same entry.
//! - [`MatchStrategy::Injective`] (default): distinct threads get **distinct**
//!   entries, minimising the sum of ΔE00 (Kuhn–Munkres). When there are more
//!   distinct threads than free entries, the closest threads are merged first.
//!
//! In both strategies, a thread whose own reference designates a palette
//! entry keeps it ("exact" match, ΔE ignored), and equal costs are broken in
//! favour of lower palette indices.

// Every index used here is a position this module computed in the very
// vectors it indexes (threads, palette, groups, cost matrix): bounds hold by
// construction, and the property tests exercise them.
#![allow(clippy::indexing_slicing)]

use std::collections::BTreeMap;

use crate::color::{DeltaE, Lab, Rgb, delta_e_2000};
use crate::thread::Thread;

/// How threads are matched to palette entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MatchStrategy {
    /// Smallest ΔE00 per thread; distinct threads may share an entry.
    Nearest,
    /// Distinct threads get distinct entries, minimising the total ΔE00.
    #[default]
    Injective,
}

/// The palette entry chosen for one thread.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PaletteMatch {
    /// Position of the entry in the palette slice.
    pub index: usize,
    /// `None` for an exact match on the thread's reference, otherwise the
    /// CIEDE2000 difference to the entry's colour.
    pub delta_e: Option<DeltaE>,
}

/// Matches every thread to a palette entry. `exact` returns the palette
/// position designated by a thread's own reference, if any.
///
/// Returns `None` if the palette is empty; otherwise one match per thread,
/// in order, each with `index < palette.len()`.
///
/// ```
/// use emb_core::{MatchStrategy, Rgb, Thread, match_to_palette};
///
/// let palette = [Rgb::new(0, 0, 255), Rgb::new(0, 0, 200), Rgb::new(255, 0, 0)];
/// let threads = [
///     Thread::from_color(Rgb::new(0, 0, 250)),
///     Thread::from_color(Rgb::new(0, 0, 245)),
/// ];
/// let nearest = match_to_palette(&threads, &palette, |_| None, MatchStrategy::Nearest).unwrap();
/// assert_eq!((nearest[0].index, nearest[1].index), (0, 0));
/// let injective = match_to_palette(&threads, &palette, |_| None, MatchStrategy::Injective).unwrap();
/// assert_ne!(injective[0].index, injective[1].index);
/// ```
pub fn match_to_palette(
    threads: &[Thread],
    palette: &[Rgb],
    exact: impl Fn(&Thread) -> Option<usize>,
    strategy: MatchStrategy,
) -> Option<Vec<PaletteMatch>> {
    if palette.is_empty() {
        return None;
    }
    let palette_lab: Vec<Lab> = palette.iter().map(|c| c.to_lab()).collect();
    let (unique, block_to_unique) = unique_threads(threads);
    let exact: Vec<Option<usize>> = unique
        .iter()
        .map(|t| exact(t).filter(|&i| i < palette.len()))
        .collect();
    let colors: Vec<Lab> = unique.iter().map(|t| t.color.to_lab()).collect();
    let free: Vec<usize> = (0..unique.len()).filter(|&u| exact[u].is_none()).collect();
    let chosen = match strategy {
        MatchStrategy::Nearest => free
            .iter()
            .map(|&u| (u, nearest(colors[u], &palette_lab, &all(palette.len()))))
            .collect(),
        MatchStrategy::Injective => assign_injectively(&free, &colors, &palette_lab, &exact),
    };
    let unique_matches: Vec<PaletteMatch> = (0..unique.len())
        .map(|u| {
            if let Some(index) = exact[u] {
                return PaletteMatch {
                    index,
                    delta_e: None,
                };
            }
            let index = chosen.get(&u).copied().unwrap_or(0);
            PaletteMatch {
                index,
                delta_e: Some(delta_e_2000(colors[u], palette_lab[index])),
            }
        })
        .collect();
    Some(
        block_to_unique
            .into_iter()
            .map(|u| unique_matches[u])
            .collect(),
    )
}

type ThreadKey = (Rgb, Option<String>, Option<String>);

/// Distinct threads (same colour and reference = same thread, see
/// [`Thread::is_same_thread`]) in order of first use, and for each input
/// thread the position of its distinct thread.
fn unique_threads(threads: &[Thread]) -> (Vec<&Thread>, Vec<usize>) {
    let mut positions: BTreeMap<ThreadKey, usize> = BTreeMap::new();
    let mut unique = Vec::new();
    let mapping = threads
        .iter()
        .map(|t| {
            let key = (t.color, t.brand.clone(), t.catalog_number.clone());
            *positions.entry(key).or_insert_with(|| {
                unique.push(t);
                unique.len() - 1
            })
        })
        .collect();
    (unique, mapping)
}

fn all(len: usize) -> Vec<usize> {
    (0..len).collect()
}

/// Entry of `candidates` with the smallest ΔE00 (first one on ties).
fn nearest(color: Lab, palette: &[Lab], candidates: &[usize]) -> usize {
    let mut best = (candidates.first().copied().unwrap_or(0), f64::INFINITY);
    for &index in candidates {
        let delta = delta_e_2000(color, palette[index]).get();
        if delta < best.1 {
            best = (index, delta);
        }
    }
    best.0
}

/// Injective assignment of the `free` threads to the entries not reserved by
/// exact matches; returns `thread → palette position`.
fn assign_injectively(
    free: &[usize],
    colors: &[Lab],
    palette: &[Lab],
    exact: &[Option<usize>],
) -> BTreeMap<usize, usize> {
    let reserved: Vec<usize> = exact.iter().flatten().copied().collect();
    let available: Vec<usize> = (0..palette.len())
        .filter(|i| !reserved.contains(i))
        .collect();
    if available.is_empty() {
        // Every entry is taken by an exact match: fall back to the nearest.
        return free
            .iter()
            .map(|&u| (u, nearest(colors[u], palette, &all(palette.len()))))
            .collect();
    }
    let groups = merge_closest(free, colors, available.len());
    let costs: Vec<Vec<i64>> = groups
        .iter()
        .map(|group| {
            available
                .iter()
                .map(|&entry| {
                    let total: f64 = group
                        .iter()
                        .map(|&u| delta_e_2000(colors[u], palette[entry]).get())
                        .sum();
                    tie_broken_cost(total, entry, groups.len(), palette.len())
                })
                .collect()
        })
        .collect();
    let assignment = hungarian(&costs);
    groups
        .iter()
        .zip(assignment)
        .flat_map(|(group, column)| {
            let entry = available[column];
            group.iter().map(move |&u| (u, entry))
        })
        .collect()
}

/// ΔE resolution of the integer costs.
const COST_SCALE: f64 = 10_000.0;

/// Integer cost: total ΔE00 (to 1e-4), then the palette position, so that
/// equal ΔE totals are decided in favour of lower indices.
fn tie_broken_cost(total_delta_e: f64, entry: usize, rows: usize, entries: usize) -> i64 {
    let scaled = (total_delta_e * COST_SCALE).round();
    // Finite and below 2^53: the conversion is exact. A non-finite ΔE cannot
    // happen with finite Lab inputs; it is mapped to a prohibitive cost.
    #[allow(clippy::cast_possible_truncation)]
    let scaled = if scaled.is_finite() && scaled < 9.0e15 {
        scaled as i64
    } else {
        i64::MAX / 4
    };
    let weight = i64::try_from(rows.saturating_mul(entries).saturating_add(1)).unwrap_or(i64::MAX);
    scaled
        .saturating_mul(weight)
        .saturating_add(i64::try_from(entry).unwrap_or(i64::MAX))
}

/// Groups the threads so that there are at most `limit` groups, by
/// repeatedly merging the two groups whose closest members have the smallest
/// ΔE00 (ties: earliest groups).
fn merge_closest(threads: &[usize], colors: &[Lab], limit: usize) -> Vec<Vec<usize>> {
    let mut groups: Vec<Vec<usize>> = threads.iter().map(|&u| vec![u]).collect();
    while groups.len() > limit.max(1) {
        let mut best = (0, 1, f64::INFINITY);
        for i in 0..groups.len() {
            for j in i + 1..groups.len() {
                let distance = group_distance(&groups[i], &groups[j], colors);
                if distance < best.2 {
                    best = (i, j, distance);
                }
            }
        }
        let merged = groups.remove(best.1);
        groups[best.0].extend(merged);
    }
    groups
}

fn group_distance(a: &[usize], b: &[usize], colors: &[Lab]) -> f64 {
    a.iter()
        .flat_map(|&i| {
            b.iter()
                .map(move |&j| delta_e_2000(colors[i], colors[j]).get())
        })
        .fold(f64::INFINITY, f64::min)
}

/// Minimum-cost assignment of every row to a distinct column (rows ≤
/// columns), by the Kuhn–Munkres algorithm with potentials, O(n²·m).
/// Returns the column of each row.
fn hungarian(costs: &[Vec<i64>]) -> Vec<usize> {
    let rows = costs.len();
    let columns = costs.first().map_or(0, Vec::len);
    if rows == 0 || columns < rows {
        return Vec::new();
    }
    let cost = |i: usize, j: usize| costs[i - 1][j - 1];
    let mut u = vec![0_i64; rows + 1];
    let mut v = vec![0_i64; columns + 1];
    let mut owner = vec![0_usize; columns + 1];
    let mut way = vec![0_usize; columns + 1];
    for row in 1..=rows {
        owner[0] = row;
        let mut j0 = 0;
        let mut min_value = vec![i64::MAX; columns + 1];
        let mut used = vec![false; columns + 1];
        loop {
            used[j0] = true;
            let i0 = owner[j0];
            let mut delta = i64::MAX;
            let mut j1 = 0;
            for j in 1..=columns {
                if used[j] {
                    continue;
                }
                let current = cost(i0, j) - u[i0] - v[j];
                if current < min_value[j] {
                    min_value[j] = current;
                    way[j] = j0;
                }
                if min_value[j] < delta {
                    delta = min_value[j];
                    j1 = j;
                }
            }
            for j in 0..=columns {
                if used[j] {
                    u[owner[j]] += delta;
                    v[j] -= delta;
                } else {
                    min_value[j] -= delta;
                }
            }
            j0 = j1;
            if owner[j0] == 0 {
                break;
            }
        }
        loop {
            let j1 = way[j0];
            owner[j0] = owner[j1];
            j0 = j1;
            if j0 == 0 {
                break;
            }
        }
    }
    let mut assignment = vec![0; rows];
    for j in 1..=columns {
        if owner[j] != 0 {
            assignment[owner[j] - 1] = j - 1;
        }
    }
    assignment
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn thread(r: u8, g: u8, b: u8) -> Thread {
        Thread::from_color(Rgb::new(r, g, b))
    }

    fn run(threads: &[Thread], palette: &[Rgb], strategy: MatchStrategy) -> Vec<usize> {
        match_to_palette(threads, palette, |_| None, strategy)
            .unwrap()
            .iter()
            .map(|m| m.index)
            .collect()
    }

    #[test]
    fn empty_palette_gives_no_match() {
        assert!(
            match_to_palette(&[thread(0, 0, 0)], &[], |_| None, MatchStrategy::Injective).is_none()
        );
    }

    #[test]
    fn close_threads_get_distinct_entries_only_when_injective() {
        let palette = [
            Rgb::new(10, 80, 120),
            Rgb::new(20, 120, 130),
            Rgb::new(250, 250, 250),
        ];
        let both_near_one = [thread(28, 126, 130), thread(22, 118, 128)];
        assert_eq!(
            run(&both_near_one, &palette, MatchStrategy::Nearest),
            vec![1, 1]
        );
        let injective = run(&both_near_one, &palette, MatchStrategy::Injective);
        assert_ne!(injective[0], injective[1]);
    }

    #[test]
    fn same_thread_in_several_blocks_keeps_one_entry() {
        let palette = [Rgb::new(0, 0, 0), Rgb::new(255, 255, 255)];
        let threads = [thread(0, 0, 0), thread(250, 250, 250), thread(0, 0, 0)];
        assert_eq!(
            run(&threads, &palette, MatchStrategy::Injective),
            vec![0, 1, 0]
        );
    }

    #[test]
    fn exact_references_are_kept_and_reserved() {
        let palette = [Rgb::new(0, 0, 0), Rgb::new(10, 10, 10), Rgb::new(255, 0, 0)];
        let mut referenced = thread(255, 0, 0);
        referenced.catalog_number = Some("ref-0".into());
        let threads = [referenced, thread(0, 0, 0)];
        let exact = |t: &Thread| (t.catalog_number.as_deref() == Some("ref-0")).then_some(0);
        let matches =
            match_to_palette(&threads, &palette, exact, MatchStrategy::Injective).unwrap();
        assert_eq!(
            matches[0],
            PaletteMatch {
                index: 0,
                delta_e: None
            }
        );
        assert_eq!(
            matches[1].index, 1,
            "entry 0 is reserved by the exact match"
        );
    }

    #[test]
    fn equal_costs_prefer_the_lowest_index() {
        let palette = [
            Rgb::new(9, 9, 9),
            Rgb::new(50, 50, 50),
            Rgb::new(50, 50, 50),
        ];
        for strategy in [MatchStrategy::Nearest, MatchStrategy::Injective] {
            assert_eq!(run(&[thread(50, 50, 50)], &palette, strategy), vec![1]);
        }
    }

    #[test]
    fn more_threads_than_entries_merges_the_closest_first() {
        let palette = [Rgb::new(0, 0, 0), Rgb::new(255, 255, 255)];
        let threads = [
            thread(250, 250, 250),
            thread(5, 5, 5),
            thread(255, 255, 250),
        ];
        let indices = run(&threads, &palette, MatchStrategy::Injective);
        assert_eq!(indices, vec![1, 0, 1]);
    }

    #[test]
    fn hungarian_solves_a_known_instance() {
        let costs = vec![vec![4, 1, 3], vec![2, 0, 5], vec![3, 2, 2]];
        assert_eq!(hungarian(&costs), vec![1, 0, 2]);
    }

    /// Minimum total cost over every injective assignment (small sizes only).
    fn brute_force(costs: &[Vec<i64>], row: usize, used: &mut Vec<bool>) -> i64 {
        if row == costs.len() {
            return 0;
        }
        let mut best = i64::MAX;
        for column in 0..used.len() {
            if !used[column] {
                used[column] = true;
                let rest = brute_force(costs, row + 1, used);
                best = best.min(costs[row][column].saturating_add(rest));
                used[column] = false;
            }
        }
        best
    }

    proptest! {
        #[test]
        fn hungarian_matches_brute_force(
            (rows, columns, flat) in (1..5_usize, 0..3_usize).prop_flat_map(|(rows, extra)| {
                let columns = rows + extra;
                (Just(rows), Just(columns), prop::collection::vec(0..1000_i64, rows * columns))
            })
        ) {
            let costs: Vec<Vec<i64>> = flat.chunks(columns).map(<[i64]>::to_vec).collect();
            let assignment = hungarian(&costs);
            let mut seen = assignment.clone();
            seen.sort_unstable();
            seen.dedup();
            prop_assert_eq!(seen.len(), rows, "not injective: {:?}", assignment);
            let total: i64 = assignment.iter().enumerate().map(|(r, &c)| costs[r][c]).sum();
            prop_assert_eq!(total, brute_force(&costs, 0, &mut vec![false; columns]));
        }

        #[test]
        fn injective_matching_is_injective_and_deterministic(
            colors in prop::collection::vec(any::<[u8; 3]>(), 1..12),
            palette in prop::collection::vec(any::<[u8; 3]>(), 12..20),
        ) {
            let threads: Vec<_> = colors.iter().map(|&[r, g, b]| thread(r, g, b)).collect();
            let palette: Vec<_> = palette.iter().map(|&[r, g, b]| Rgb::new(r, g, b)).collect();
            let first = run(&threads, &palette, MatchStrategy::Injective);
            prop_assert_eq!(&first, &run(&threads, &palette, MatchStrategy::Injective));
            for (i, a) in threads.iter().enumerate() {
                for (j, b) in threads.iter().enumerate() {
                    if !a.is_same_thread(b) {
                        prop_assert_ne!(first[i], first[j]);
                    }
                }
            }
        }

        /// The injective total ΔE is never above any other injective choice:
        /// in particular, never above the nearest choice when the latter
        /// happens to be injective.
        #[test]
        fn injective_matching_is_optimal_when_nearest_is_injective(
            colors in prop::collection::vec(any::<[u8; 3]>(), 1..8),
            palette in prop::collection::vec(any::<[u8; 3]>(), 8..16),
        ) {
            let threads: Vec<_> = colors.iter().map(|&[r, g, b]| thread(r, g, b)).collect();
            let palette: Vec<_> = palette.iter().map(|&[r, g, b]| Rgb::new(r, g, b)).collect();
            // Total ΔE over distinct threads (each counted once).
            let total = |strategy| -> f64 {
                let matches = match_to_palette(&threads, &palette, |_| None, strategy).unwrap();
                let (unique, map) = unique_threads(&threads);
                let mut per_unique = vec![0.0; unique.len()];
                for (&u, m) in map.iter().zip(&matches) {
                    per_unique[u] = m.delta_e.unwrap().get();
                }
                per_unique.iter().sum()
            };
            let nearest = run(&threads, &palette, MatchStrategy::Nearest);
            let (unique, map) = unique_threads(&threads);
            let mut per_unique = vec![None; unique.len()];
            for (&u, &index) in map.iter().zip(&nearest) {
                per_unique[u] = Some(index);
            }
            let mut indices: Vec<_> = per_unique.iter().flatten().copied().collect();
            indices.sort_unstable();
            indices.dedup();
            if indices.len() == unique.len() {
                prop_assert!(total(MatchStrategy::Injective) <= total(MatchStrategy::Nearest) + 1e-3);
            }
        }
    }
}

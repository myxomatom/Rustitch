//! Thread fallbacks for files that store no or inconsistent colour information.

use emb_core::{Rgb, Thread};

use crate::warning::Warning;

/// Contrasting colours assigned, in order, to blocks of colourless designs.
/// Source: Sasha Trubetskoy, "List of 20 Simple, Distinct Colors" (subset).
const DEFAULT_COLORS: [Rgb; 12] = [
    Rgb::new(0x00, 0x00, 0x00),
    Rgb::new(0xE6, 0x19, 0x4B),
    Rgb::new(0x3C, 0xB4, 0x4B),
    Rgb::new(0x43, 0x63, 0xD8),
    Rgb::new(0xF5, 0x82, 0x31),
    Rgb::new(0x91, 0x1E, 0xB4),
    Rgb::new(0x42, 0xD4, 0xF4),
    Rgb::new(0xF0, 0x32, 0xE6),
    Rgb::new(0xBF, 0xEF, 0x45),
    Rgb::new(0x9A, 0x63, 0x24),
    Rgb::new(0x00, 0x00, 0x75),
    Rgb::new(0xFF, 0xE1, 0x19),
];

/// Default thread for colour block `block`, cycling through a contrasting palette.
#[must_use]
pub(crate) fn default_thread(block: usize) -> Thread {
    let color = DEFAULT_COLORS
        .get(block % DEFAULT_COLORS.len())
        .copied()
        .unwrap_or(Rgb::new(0, 0, 0));
    Thread::from_color(color)
}

/// Makes `threads` hold exactly one thread per block, reporting any fix.
pub(crate) fn reconcile_threads(
    mut threads: Vec<Thread>,
    blocks: usize,
    warnings: &mut Vec<Warning>,
) -> Vec<Thread> {
    if threads.len() == blocks {
        return threads;
    }
    if threads.is_empty() {
        warnings.push(Warning::DefaultColorsAssigned { blocks });
    } else {
        warnings.push(Warning::ThreadCountMismatch {
            declared: threads.len(),
            blocks,
        });
    }
    threads.truncate(blocks);
    let missing = threads.len()..blocks;
    threads.extend(missing.map(default_thread));
    threads
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_threads_get_contrasting_defaults() {
        let mut warnings = Vec::new();
        let threads = reconcile_threads(vec![], 3, &mut warnings);
        assert_eq!(threads.len(), 3);
        assert_ne!(threads[0].color, threads[1].color);
        assert_eq!(warnings, vec![Warning::DefaultColorsAssigned { blocks: 3 }]);
    }

    #[test]
    fn extra_threads_are_dropped_with_a_warning() {
        let mut warnings = Vec::new();
        let threads = reconcile_threads(vec![default_thread(5); 4], 2, &mut warnings);
        assert_eq!(threads.len(), 2);
        assert_eq!(
            warnings,
            vec![Warning::ThreadCountMismatch {
                declared: 4,
                blocks: 2
            }]
        );
    }

    #[test]
    fn default_palette_cycles() {
        assert_eq!(default_thread(0), default_thread(DEFAULT_COLORS.len()));
    }
}

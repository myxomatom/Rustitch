//! Stitches: needle positions tagged with the machine action to perform.

use crate::units::Point;

/// The machine action associated with a needle position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum StitchKind {
    /// Move to the position with the needle down: a visible stitch.
    Normal,
    /// Move to the position without stitching.
    Jump,
    /// Cut the thread. The position is the needle position when cutting.
    Trim,
    /// Stop the machine and switch to the next colour block's thread.
    ColorChange,
    /// Stop the machine without changing colour block (appliqué, manual action…).
    Stop,
    /// Move to the position and drop a sequin.
    SequinEject,
    /// End of the design. Only allowed as the very last stitch.
    End,
}

impl StitchKind {
    /// Whether this kind moves the needle to its position (as opposed to a
    /// command executed at the current needle position).
    ///
    /// ```
    /// use emb_core::StitchKind;
    ///
    /// assert!(StitchKind::Jump.is_move());
    /// assert!(!StitchKind::Trim.is_move());
    /// ```
    #[must_use]
    pub const fn is_move(self) -> bool {
        matches!(self, Self::Normal | Self::Jump | Self::SequinEject)
    }
}

/// A needle position (absolute, in millimetres) and its action.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stitch {
    /// What the machine does at this position.
    pub kind: StitchKind,
    /// Absolute position.
    pub position: Point,
}

impl Stitch {
    /// Creates a stitch.
    #[must_use]
    pub const fn new(kind: StitchKind, position: Point) -> Self {
        Self { kind, position }
    }
}

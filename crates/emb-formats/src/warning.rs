//! Non-fatal issues: everything a conversion approximated, dropped or guessed.

use std::fmt;

use emb_core::DeltaE;

/// A loss, approximation or anomaly detected while reading or writing.
#[derive(Debug, Clone, PartialEq)]
pub enum Warning {
    /// The data stops in the middle of a record.
    TruncatedData {
        /// Byte offset of the incomplete record.
        offset: usize,
    },
    /// The stitch data has no end marker.
    MissingEndMarker,
    /// An unknown command was found; reading stopped there.
    UnknownCommand {
        /// Byte offset of the command.
        offset: usize,
        /// Command code.
        code: u8,
    },
    /// The file declares a number of threads different from its colour blocks.
    ThreadCountMismatch {
        /// Threads declared by the file.
        declared: usize,
        /// Colour blocks found in the stitch data.
        blocks: usize,
    },
    /// The file's own thread list could not be read; palette colours were used.
    ThreadListUnreadable,
    /// The file declares a version this reader does not know; it was read
    /// as the closest known version.
    UnknownVersion {
        /// Version string found in the file.
        version: String,
    },
    /// The file stores no colour: a default contrasting palette was used.
    DefaultColorsAssigned {
        /// Number of colour blocks that received a default colour.
        blocks: usize,
    },
    /// A colour block refers to an unknown palette entry.
    UnknownPaletteIndex {
        /// Colour block.
        block: usize,
        /// Palette index found in the file.
        index: i64,
    },
    /// The format cannot store stops: they became colour changes to the same thread.
    StopsConvertedToColorChanges {
        /// Number of stops converted.
        count: usize,
    },
    /// The format cannot store sequins: sequin ejections became jumps.
    SequinsConvertedToJumps {
        /// Number of sequins converted.
        count: usize,
    },
    /// Runs of jumps that the machine trims anyway now carry an explicit trim.
    ImplicitTrimsInserted {
        /// Number of trims inserted.
        count: usize,
    },
    /// The format has no trim command: trims were not written.
    TrimsNotStored {
        /// Number of trims dropped.
        count: usize,
    },
    /// The target format stores no colour information.
    ColorsNotStored {
        /// Number of colour blocks whose thread is lost.
        blocks: usize,
    },
    /// A thread was replaced by a palette entry whose colour differs noticeably.
    ThreadApproximated {
        /// Colour block.
        block: usize,
        /// Palette index used.
        palette_index: u8,
        /// CIEDE2000 difference between the thread and the palette entry.
        delta_e: DeltaE,
    },
    /// Distinct threads received the same palette entry.
    ThreadsShareColor {
        /// Palette index shared.
        palette_index: u8,
        /// Colour blocks concerned.
        blocks: Vec<usize>,
    },
    /// Brands or non-numeric catalogue references could not be stored.
    ThreadDetailsNotStored {
        /// Colour blocks concerned.
        blocks: Vec<usize>,
    },
    /// A thread name was shortened or sanitised to fit the format.
    ThreadNameAltered {
        /// Colour block.
        block: usize,
        /// Name of the thread.
        original: String,
        /// Name written to the file.
        written: String,
    },
    /// Data after the end of the design was not understood and was ignored.
    UnreadTrailingData {
        /// Byte offset of the data.
        offset: usize,
        /// Number of bytes ignored.
        bytes: usize,
    },
    /// The design name was shortened or sanitised to fit the format.
    NameAltered {
        /// Name in the design.
        original: String,
        /// Name written to the file.
        written: String,
    },
}

impl fmt::Display for Warning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TruncatedData { offset } => {
                write!(
                    f,
                    "data truncated at byte {offset}; the end of the design is lost"
                )
            }
            Self::MissingEndMarker => write!(f, "no end marker; the design may be incomplete"),
            Self::UnknownCommand { offset, code } => write!(
                f,
                "unknown command 0x{code:02X} at byte {offset}; reading stopped there"
            ),
            Self::ThreadCountMismatch { declared, blocks } => write!(
                f,
                "file declares {declared} thread(s) for {blocks} colour block(s)"
            ),
            Self::ThreadListUnreadable => write!(
                f,
                "the thread list could not be read; palette colours used instead"
            ),
            Self::UnknownVersion { version } => {
                write!(
                    f,
                    "unknown format version {version:?}; read as a known version"
                )
            }
            Self::DefaultColorsAssigned { blocks } => write!(
                f,
                "no colour information: default colours assigned to {blocks} block(s)"
            ),
            Self::UnknownPaletteIndex { block, index } => write!(
                f,
                "colour block {block} uses unknown palette index {index}; default colour used"
            ),
            Self::StopsConvertedToColorChanges { count } => write!(
                f,
                "{count} stop(s) converted to colour change(s) to the same thread"
            ),
            Self::SequinsConvertedToJumps { count } => {
                write!(
                    f,
                    "{count} sequin(s) not supported by the format, written as jumps"
                )
            }
            Self::ImplicitTrimsInserted { count } => write!(
                f,
                "{count} run(s) of jumps trigger an automatic trim; explicit trim(s) added"
            ),
            Self::TrimsNotStored { count } => write!(
                f,
                "{count} trim(s) not written: the format has no trim command"
            ),
            Self::ColorsNotStored { blocks } => write!(
                f,
                "the format stores no colour: thread information of {blocks} block(s) is lost"
            ),
            Self::ThreadApproximated {
                block,
                palette_index,
                delta_e,
            } => write!(
                f,
                "colour block {block}: written as palette colour {palette_index}, ΔE00 {delta_e} from the thread"
            ),
            Self::ThreadsShareColor {
                palette_index,
                blocks,
            } => write!(
                f,
                "distinct threads of blocks {blocks:?} all use palette colour {palette_index}"
            ),
            Self::ThreadDetailsNotStored { blocks } => write!(
                f,
                "brand or non-numeric reference of the threads of blocks {blocks:?} not stored"
            ),
            Self::ThreadNameAltered {
                block,
                original,
                written,
            } => write!(
                f,
                "colour block {block}: thread name {original:?} written as {written:?}"
            ),
            Self::UnreadTrailingData { offset, bytes } => write!(
                f,
                "{bytes} byte(s) after the end of the design (offset {offset}) ignored"
            ),
            Self::NameAltered { original, written } => {
                write!(f, "design name {original:?} written as {written:?}")
            }
        }
    }
}

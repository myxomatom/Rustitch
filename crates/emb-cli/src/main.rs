//! `emb`: command-line embroidery file converter.
//!
//! Exit status: 0 on success, 3 on success with warnings (something was
//! approximated or lost), 1 on failure, 2 on invalid usage.

#![forbid(unsafe_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, anyhow};
use clap::{Args, Parser, Subcommand, ValueEnum};
use emb_core::{MatchStrategy, StitchPattern, Timestamp};
use emb_formats::convert::{self, ConvertError, ConvertRequest, Loaded, WriteOptions};
use emb_formats::jef::Placement;
use emb_formats::{Format, Warning};

/// Exit status of a successful run that reported warnings.
const EXIT_WARNINGS: u8 = 3;

#[derive(Debug, Parser)]
#[command(name = "emb", version, about = "Embroidery machine file converter")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Convert a design to another machine format.
    Convert {
        /// Input file (format detected from its content).
        input: PathBuf,
        /// Output file (format taken from its extension unless --to is given).
        output: PathBuf,
        /// Output format.
        #[arg(long, value_parser = parse_format)]
        to: Option<Format>,
        #[command(flatten)]
        jef: JefOptions,
    },
    /// Show what a design file contains.
    Info {
        /// File to inspect.
        file: PathBuf,
    },
}

/// Options of the JEF writer.
#[derive(Debug, Clone, Copy, Args)]
struct JefOptions {
    /// Centre the design in the hoop (JEF output only).
    #[arg(long)]
    center: bool,
    /// Thread to palette matching (JEF output only) [default: injective].
    #[arg(long, value_enum)]
    colors: Option<ColorStrategy>,
    /// Do not append the ENHANCE block with the real threads (JEF output only).
    #[arg(long)]
    no_enhance: bool,
}

/// Thread to palette matching strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum ColorStrategy {
    /// Distinct threads get distinct palette colours (smallest total ΔE00).
    Injective,
    /// Each thread gets its nearest palette colour (may merge close threads).
    Nearest,
}

impl JefOptions {
    /// The service options set by the flags (unset flags keep the defaults).
    fn write_options(self) -> WriteOptions {
        WriteOptions {
            placement: self.center.then_some(Placement::Center),
            colors: self.colors.map(MatchStrategy::from),
            enhance: self.no_enhance.then_some(false),
        }
    }
}

impl From<ColorStrategy> for MatchStrategy {
    fn from(strategy: ColorStrategy) -> Self {
        match strategy {
            ColorStrategy::Injective => MatchStrategy::Injective,
            ColorStrategy::Nearest => MatchStrategy::Nearest,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Outcome {
    Clean,
    Warnings,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match &cli.command {
        Command::Convert {
            input,
            output,
            to,
            jef,
        } => convert(input, output, *to, *jef),
        Command::Info { file } => info(file),
    };
    match result {
        Ok(Outcome::Clean) => ExitCode::SUCCESS,
        Ok(Outcome::Warnings) => ExitCode::from(EXIT_WARNINGS),
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::FAILURE
        }
    }
}

fn parse_format(text: &str) -> Result<Format, String> {
    Format::from_extension(text).ok_or_else(|| {
        let known: Vec<_> = Format::ALL.iter().map(|f| f.extension()).collect();
        format!("unknown format {text:?} (known: {})", known.join(", "))
    })
}

/// Extension of `path`, as given to the conversion service as a hint.
fn extension(path: &Path) -> Option<&str> {
    path.extension().and_then(|e| e.to_str())
}

fn read_file(path: &Path) -> Result<Vec<u8>> {
    fs::read(path).with_context(|| format!("cannot read {}", path.display()))
}

/// Presents service errors in terms of command-line flags.
fn cli_error(error: ConvertError) -> anyhow::Error {
    match error {
        ConvertError::OptionNotApplicable {
            option, applies_to, ..
        } => {
            let flag = match option {
                "placement" => "center",
                "enhance" => "no-enhance",
                other => other,
            };
            anyhow!("--{flag} only applies to {applies_to} output")
        }
        other => other.into(),
    }
}

fn note_extension_mismatch(path: &Path, loaded: &Loaded) {
    if loaded.extension_mismatch.is_some() {
        eprintln!(
            "note: {} contains {} data despite its extension",
            path.display(),
            loaded.format
        );
    }
}

fn report(stage: &str, warnings: &[Warning]) -> Outcome {
    for warning in warnings {
        eprintln!("warning ({stage}): {warning}");
    }
    if warnings.is_empty() {
        Outcome::Clean
    } else {
        Outcome::Warnings
    }
}

fn now() -> Option<Timestamp> {
    let seconds = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs();
    Timestamp::from_unix_seconds(i64::try_from(seconds).ok()?)
}

fn convert(input: &Path, output: &Path, to: Option<Format>, jef: JefOptions) -> Result<Outcome> {
    let target = convert::resolve_target(to, extension(output)).map_err(|_| {
        anyhow!(
            "cannot infer the output format from {}; use --to",
            output.display()
        )
    })?;
    let request = ConvertRequest {
        target,
        options: jef.write_options(),
        default_date: now(),
    };
    let data = read_file(input)?;
    let conversion = convert::convert(&data, extension(input), &request)
        .map_err(cli_error)
        .with_context(|| format!("cannot convert {}", input.display()))?;
    note_extension_mismatch(input, &conversion.source);
    fs::write(output, &conversion.bytes)
        .with_context(|| format!("cannot write {}", output.display()))?;
    println!(
        "{} ({}) -> {} ({target}): {}",
        input.display(),
        conversion.source.format,
        output.display(),
        summary(&conversion.pattern)
    );
    let read_outcome = report("read", &conversion.source.output.warnings);
    let write_outcome = report("write", &conversion.write_warnings);
    Ok(
        if read_outcome == Outcome::Clean && write_outcome == Outcome::Clean {
            Outcome::Clean
        } else {
            Outcome::Warnings
        },
    )
}

fn summary(pattern: &StitchPattern) -> String {
    let stats = pattern.stats();
    let size = pattern.bounds().map_or_else(
        || "empty".to_owned(),
        |b| format!("{:.1} x {:.1} mm", b.width().get(), b.height().get()),
    );
    format!(
        "{} stitches, {} colour block(s), {size}",
        stats.normal,
        pattern.threads().len()
    )
}

fn info(path: &Path) -> Result<Outcome> {
    let data = read_file(path)?;
    let loaded = convert::load(&data, extension(path))
        .map_err(cli_error)
        .with_context(|| format!("cannot read {}", path.display()))?;
    note_extension_mismatch(path, &loaded);
    let format = loaded.format;
    let read = loaded.output;
    let pattern = &read.pattern;
    let meta = &pattern.metadata;
    println!("File:     {}", path.display());
    println!("Format:   {format}");
    let optional = [
        ("Name", meta.name.clone()),
        ("Author", meta.author.clone()),
        ("Copyright", meta.copyright.clone()),
        ("Date", meta.date.map(|d| d.to_string())),
        (
            "Hoop",
            meta.hoop
                .map(|h| format!("{} x {} mm", h.width.get(), h.height.get())),
        ),
    ];
    for (label, value) in optional {
        if let Some(value) = value {
            println!("{:<9} {value}", format!("{label}:"));
        }
    }
    println!("Design:   {}", summary(pattern));
    let s = pattern.stats();
    println!(
        "Commands: {} jump(s), {} trim(s), {} colour change(s), {} stop(s), {} sequin(s)",
        s.jumps, s.trims, s.color_changes, s.stops, s.sequins
    );
    println!("Threads:");
    for (i, thread) in pattern.threads().iter().enumerate() {
        let details: Vec<_> = [&thread.name, &thread.brand, &thread.catalog_number]
            .into_iter()
            .flatten()
            .map(String::as_str)
            .collect();
        println!("  {:>3}. {}  {}", i + 1, thread.color, details.join(" "));
    }
    Ok(report("read", &read.warnings))
}

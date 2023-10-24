use std::{io::Write, path::PathBuf, thread, time::Duration};

use anyhow::{bail, Result};
use clap::Parser;
use crossbeam_channel::unbounded;
use indicatif::{ParallelProgressIterator, ProgressBar, ProgressStyle};
use itertools::Itertools;
use phylotree::tree::Tree;
use rayon::prelude::*;

mod csv;
mod io;

#[derive(Parser)]
/// Compare trees to reference trees
struct Cli {
    /// Directory containing reference trees
    ref_trees: PathBuf,
    /// Directory containing trees to compare
    cmp_trees: Vec<PathBuf>,
    /// Output file
    #[arg(short, long)]
    output: PathBuf,
    /// Add `marker` columns to csv output, specified in JSON format
    #[arg(short, long)]
    markers: Option<String>,
    /// Compare branch lengths instead of tree metrics
    #[arg(short, long)]
    lengths: bool,
    /// Include tips when comparing branches of trees (this flag is only
    /// used when the `--lengths` flag is specified)
    #[arg(short = 't', long)]
    include_tips: bool,
    /// If specified, the program will extract pairwise distances, compare them
    /// and write the result in the specified file
    #[arg(short, long)]
    distances: Option<PathBuf>,
    /// Exit the program early on error instead of listing them at the end
    #[arg(short, long)]
    strict: bool,
    /// Number of threads to use in parallel (0 = all available threads)
    #[arg(short, long, default_value_t = 0)]
    threads: usize,
    /// Do not compress output csv using gzip
    #[arg(short, long)]
    no_compression: bool,
}

fn main() -> Result<()> {
    let args = Cli::parse();

    // Build thread-pool
    rayon::ThreadPoolBuilder::new()
        .num_threads(args.threads)
        .build_global()?;

    // Check that we have trees to compare to reference
    if args.cmp_trees.is_empty() {
        bail!("You must specify at least 1 directory to compare to the reference");
    }

    // Check that ref_trees is a directory
    io::check_dir(&args.ref_trees)?;

    // Read reference trees
    let ref_trees = io::read_refs(&args.ref_trees)?;
    eprintln!("Reference trees loaded: {}", ref_trees.len());

    // init output file
    let mut writer: Box<dyn std::io::Write> = if args.no_compression {
        Box::new(io::init_writer(args.output)?)
    } else {
        Box::new(io::init_gz_writer(args.output)?)
    };

    // Write header to output file
    let out_type = if args.lengths {
        csv::CSVType::Branches
    } else {
        csv::CSVType::Trees
    };

    let mut header = csv::get_header_string(out_type);
    let mut markers = None;
    if let Some(marker_str) = args.markers {
        let (marker_header, marker_values) = csv::parse_markers(&marker_str)?;
        header.push_str(&format!(",{marker_header}"));
        markers = Some(marker_values);
    }
    let markers = markers;

    writer.write_all((header + "\n").as_bytes())?;

    let mut errors = vec![];
    let mut not_found = vec![];
    let mut pairs = vec![];

    // Load tree pairs
    let spinner = init_spinner(ref_trees.len() as u64);
    spinner.set_message("Loading Trees");
    for pair in io::trees_iter(&args.cmp_trees[0])? {
        let (id, tree) = match pair {
            Ok(p) => p,
            Err(e) => {
                if args.strict {
                    return Err(e);
                }
                errors.push(e);
                continue;
            }
        };

        if let Some(reftree) = ref_trees.get(&id) {
            pairs.push((id, reftree.clone(), tree));
        } else {
            not_found.push(id)
        }
        spinner.inc(1)
    }
    spinner.finish_with_message("Loaded reference trees");

    // Compare trees
    let (sender, receiver) = unbounded();

    thread::spawn(move || {
        pairs
            .into_par_iter()
            .progress_count(ref_trees.len() as u64)
            .for_each_with(&sender, |sender, (id, reftree, cmptree)| {
                let res = do_comparison(
                    &id,
                    &reftree,
                    &cmptree,
                    args.lengths,
                    args.include_tips,
                    markers.as_deref(),
                );
                sender.send(res).unwrap()
            });
        drop(sender);
    });

    for record in receiver {
        writer.write_all((record? + "\n").as_bytes())?;
    }

    writer.flush()?;

    if !not_found.is_empty() {
        let n = not_found.len();
        eprintln!("Could not find reference {n} trees:");
        for tree in not_found.into_iter().take(10) {
            eprintln!("\t- {}", tree)
        }
        if n > 10 {
            eprintln!("\t- ...")
        }
    }

    if !errors.is_empty() {
        eprintln!("There were errors reading some trees:");
        for err in errors {
            eprintln!("{}", err);
        }
    }

    Ok(())
}

// The heart of the program
fn do_comparison(
    id: &str,
    reftree: &Tree,
    cmptree: &Tree,
    brlens: bool,
    include_tips: bool,
    markers: Option<&str>,
) -> Result<String> {
    let res = if brlens {
        let (refb, cmpb, common) = reftree.compare_branch_lengths(cmptree, include_tips)?;
        let ref_s = refb
            .into_iter()
            .map(|v| csv::format_branch_record(id, Some(v), None, markers))
            .join("\n")
            + "\n";

        let common_s = common
            .into_iter()
            .map(|(r, c)| csv::format_branch_record(id, Some(r), Some(c), markers))
            .join("\n")
            + "\n";

        let cmp_s = cmpb
            .into_iter()
            .map(|v| csv::format_branch_record(id, None, Some(v), markers))
            .join("\n");

        ref_s + &common_s + &cmp_s
    } else {
        reftree
            .compare_topologies(cmptree)
            .map(|c| csv::format_tree_record(id, reftree.n_leaves(), &c, markers))?
    };

    Ok(res)
}

fn init_spinner(len: u64) -> ProgressBar {
    let bar = ProgressBar::new(len);
    bar.enable_steady_tick(Duration::from_millis(80));
    let spinner_style = ProgressStyle::with_template("{spinner:.cyan} {wide_msg}")
        .unwrap()
        .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏");
    bar.set_style(spinner_style);

    bar
}

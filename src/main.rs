use std::{fs::File, io::Write, path::PathBuf, thread, time::Duration};

use anyhow::{Context, Result};
use clap::Parser;
use crossbeam_channel::unbounded;
use gzp::{deflate::Gzip, syncz::SyncZBuilder};
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
    #[clap(index = 1)]
    ref_trees: PathBuf,
    /// Directory containing trees to compare
    #[clap(index = 2)]
    cmp_trees: Vec<PathBuf>,
    /// Output file
    #[arg(short, long)]
    output: PathBuf,
    /// Compare branch lengths instead of tree metrics
    #[arg(short, long)]
    lengths: bool,
    /// If specified, the program will extract pairwise distances, compare them
    /// and write the result in the specified file
    #[arg(short, long)]
    distances: Option<PathBuf>,
    /// Exit the program early on error instead of listing them at the end
    #[arg(short, long)]
    strict: bool,
}

fn main() -> Result<()> {
    let args = Cli::parse();

    // Check that ref_trees is a directory
    io::check_dir(&args.ref_trees)?;

    // Read reference trees
    let ref_trees = io::read_refs(&args.ref_trees)?;
    eprintln!("Reference trees loaded: {}", ref_trees.len());

    // init output file
    let output_path = io::add_gz_ext(args.output);
    let output = File::create(output_path).context("Could not create output file")?;
    let mut writer = SyncZBuilder::<Gzip, _>::new().from_writer(output);

    // Write header to output file
    let out_type = if args.lengths {
        csv::CSVType::Branches
    } else {
        csv::CSVType::Trees
    };
    writer.write_all((csv::get_header_string(out_type) + "\n").as_bytes())?;

    let mut errors = vec![];
    let mut not_found = vec![];
    let mut pairs = vec![];

    // Load tree pairs
    let bar = ProgressBar::new(ref_trees.len() as u64);
    bar.enable_steady_tick(Duration::from_millis(80));
    let spinner_style = ProgressStyle::with_template("{spinner:.cyan} {wide_msg}")
        .unwrap()
        .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏");
    bar.set_style(spinner_style);
    bar.set_message("Loading Trees");
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
        bar.inc(1)
    }
    bar.finish_with_message("Loaded reference trees");

    // Compare trees
    let (sender, receiver) = unbounded();

    thread::spawn(move || {
        pairs
            .into_par_iter()
            .progress_count(ref_trees.len() as u64)
            .for_each_with(&sender, |sender, (id, reftree, cmptree)| {
                let res = do_comparison(&id, &reftree, &cmptree, args.lengths);
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
fn do_comparison(id: &str, reftree: &Tree, cmptree: &Tree, brlens: bool) -> Result<String> {
    let res = if brlens {
        let (refb, cmpb, common) = reftree.compare_branch_lengths(cmptree, false)?;
        let ref_s = refb
            .into_iter()
            .map(|v| csv::format_branch_record(id, Some(v), None))
            .join("\n")
            + "\n";

        let common_s = common
            .into_iter()
            .map(|(r, c)| csv::format_branch_record(id, Some(r), Some(c)))
            .join("\n")
            + "\n";

        let cmp_s = cmpb
            .into_iter()
            .map(|v| csv::format_branch_record(id, None, Some(v)))
            .join("\n");

        ref_s + &common_s + &cmp_s
    } else {
        reftree
            .compare_topologies(cmptree)
            .map(|c| csv::format_tree_record(id, reftree.n_leaves(), &c))?
    };

    Ok(res)
}

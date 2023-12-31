use std::{
    path::{Path, PathBuf},
    thread,
    time::Duration,
};

use anyhow::{bail, Result};
use clap::Parser;
use crossbeam_channel::unbounded;
use indicatif::{ParallelProgressIterator, ProgressBar, ProgressStyle};
use rayon::prelude::*;

mod comp;
// mod csv;
mod io;

#[derive(Parser)]
/// Compare trees to reference trees
struct Cli {
    /// Directory containing reference trees
    ref_trees: PathBuf,
    /// Directory containing trees to compare
    cmp_trees: Vec<PathBuf>,
    /// Output file prefix that will be used for all output files
    #[arg(short, long)]
    output_prefix: PathBuf,
    /// Add `marker` columns to csv output with this constant.  
    /// If unset, the column will be empty in the output file
    #[arg(short, long)]
    marker: Option<String>,
    /// Compare branch lengths instead of tree metrics
    #[arg(short, long)]
    lengths: bool,
    /// Include tips when comparing branches of trees (this flag is only
    /// used when the `--lengths` flag is specified)
    #[arg(short = 'i', long)]
    include_tips: bool,
    /// If specified compare pairwise distances
    #[arg(short, long)]
    distances: bool,
    /// If specified compare topologies
    #[arg(short, long)]
    topology: bool,
    /// If specified compare branches
    #[arg(short, long)]
    branches: bool,
    /// Compare everything: topology, branches and pairwise distances.
    #[arg(short, long)]
    all: bool,
    /// Exit the program early on error instead of listing them at the end
    #[arg(short, long)]
    strict: bool,
    /// Number of threads to use in parallel (0 = all available threads)
    #[arg(long, default_value_t = 0)]
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

    // Set up comparison mode
    let compare_topo = args.topology || args.all;
    let compare_lens = args.lengths || args.all;
    let compare_dist = args.distances || args.all;

    if !compare_topo && !compare_lens && !compare_dist {
        bail!(
            "You must specify at least one modality to compare: topology, branches, lengths or all"
        )
    }

    // Read reference trees
    let ref_trees = io::read_refs(&args.ref_trees)?;
    eprintln!("Reference trees loaded: {}", ref_trees.len());

    // init output files
    let zipped = !args.no_compression;
    let dist_path = io::get_suffixed_filenme(&args.output_prefix, "dist", "csv", zipped)?;
    let mut dist_writer = io::get_output(dist_path.clone(), zipped, compare_dist)?;

    let topo_path = io::get_suffixed_filenme(&args.output_prefix, "topo", "csv", zipped)?;
    let mut topo_writer = io::get_output(topo_path.clone(), zipped, compare_topo)?;

    let brlen_path = io::get_suffixed_filenme(&args.output_prefix, "brlen", "csv", zipped)?;
    let mut brlen_writer = io::get_output(brlen_path.clone(), zipped, compare_lens)?;

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
                let res = comp::compare_trees(
                    id,
                    &reftree,
                    &cmptree,
                    compare_topo,
                    compare_lens,
                    compare_dist,
                    args.include_tips,
                );

                match sender.send(res) {
                    Ok(_) => {}
                    Err(e) => eprintln!("Error sending: {e:?}"),
                };
            });
        drop(sender);
    });

    for record in receiver {
        let record = record?;

        if let Some(mut topo) = record.topology {
            topo.marker = args.marker.clone();
            topo_writer.as_mut().map(|w| w.serialize(topo));
        }

        if let Some(brlens) = record.branches {
            for mut brlen in brlens {
                brlen.marker = args.marker.clone();
                brlen_writer.as_mut().map(|w| w.serialize(brlen));
            }
        }

        if let Some(dists) = record.distances {
            for mut dist in dists {
                dist.marker = args.marker.clone();
                dist_writer.as_mut().map(|w| w.serialize(dist));
            }
        }
    }

    dist_writer.as_mut().map(|w| w.flush());
    brlen_writer.as_mut().map(|w| w.flush());
    topo_writer.as_mut().map(|w| w.flush());

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

    if let Some(_) = dist_writer {
        eprintln!("Wrote distance comparison to:  {}", dist_path.display())
    }
    if let Some(_) = topo_writer {
        eprintln!("Wrote topology comparison to:  {}", topo_path.display())
    }
    if let Some(_) = brlen_writer {
        eprintln!("Wrote branch   comparison to:  {}", brlen_path.display())
    }

    Ok(())
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

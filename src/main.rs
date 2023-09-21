use std::{
    fs::{metadata, File},
    io::Write,
    path::PathBuf,
};

use anyhow::{bail, Context, Result};
use clap::{Parser, ValueEnum};
use gzp::{deflate::Gzip, syncz::SyncZBuilder};
use indicatif::ProgressIterator;
use phylotree::tree::Comparison;

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
    /// Exit the program early on error instead of listing them at the end
    #[arg(short, long)]
    strict: bool,
    /// Comparison Mode for the tool
    #[arg(short, long, value_enum, default_value_t=CompMode::Topology)]
    mode: CompMode,
}

#[derive(Clone, ValueEnum)]
enum CompMode {
    Topology,
    Distances,
    Both,
    Branches,
}

const TREES_HEADER: [&str; 6] = ["id", "size", "rf", "norm_rf", "rf_weight", "kf_score"];
const DISTS_HEADER: [&str; 4] = ["id", "ref", "comp", "diff"];
const BRLNS_HEADER: [&str; 3] = ["id", "type", "length"];

fn main() -> Result<()> {
    let args = Cli::parse();

    // Check that ref_trees is a directory
    if !metadata(&args.ref_trees)
        .context(format!(
            "Could not read directory: {}",
            args.ref_trees.display()
        ))?
        .is_dir()
    {
        bail!("{} is not a directory", args.ref_trees.display());
    }

    // Read reference trees
    let ref_trees = io::read_refs(&args.ref_trees)?;
    eprintln!("Reference trees loaded: {}", ref_trees.len());

    // init output file
    let output = File::create(args.output).context("Could not create output file")?;
    let mut writer = SyncZBuilder::<Gzip, _>::new().from_writer(output);

    // Write header to output file
    writer.write_all((TREES_HEADER.join(",") + "\n").as_bytes())?;

    let mut errors = vec![];
    let mut not_found = vec![];

    for pair in io::trees_iter(&args.cmp_trees[0])?.progress_count(ref_trees.len() as u64) {
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

        match args.mode {
            CompMode::Branches => eprintln!("Comparing branches"),
            _ => {
                if let Some(reftree) = ref_trees.get(&id) {
                    let cmp = reftree.compare_topologies(&tree)?;
                    writer.write_all(
                        (format_record(&id, reftree.n_leaves(), &cmp) + "\n").as_bytes(),
                    )?;
                } else {
                    not_found.push(id)
                }
            }
        }
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

fn format_record(id: &str, size: usize, cmp: &Comparison) -> String {
    format!(
        "{id},{size},{},{},{},{}",
        cmp.rf, cmp.norm_rf, cmp.weighted_rf, cmp.branch_score
    )
}

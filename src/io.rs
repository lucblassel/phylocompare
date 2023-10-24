use anyhow::{bail, Context, Result};
use gzp::{deflate::Gzip, syncz::SyncZBuilder};
use phylotree::tree::Tree;
use std::{
    collections::HashMap,
    ffi::{OsStr, OsString},
    fs::{self, metadata, File},
    io::BufWriter,
    path::{Path, PathBuf},
};

/// Check if path exists and is a directory
pub fn check_dir(path: &Path) -> Result<()> {
    if !metadata(path)
        .context(format!("Could not read directory: {}", path.display()))?
        .is_dir()
    {
        bail!("{} is not a directory", path.display());
    }

    Ok(())
}

// Check if file extensions match newick ones
pub fn is_newick(path: &Path) -> bool {
    let ext = path.extension().and_then(OsStr::to_str);
    ext == Some("nwk") || ext == Some("newick")
}

// Extract file stem as an identifier
pub fn get_file_id(path: &Path) -> Result<String> {
    let id = path
        .file_stem()
        .and_then(OsStr::to_str)
        .context(format!("Could not extract ID from: {}", path.display()))?;

    Ok(id
        .split('.')
        .next()
        .context(format!("Could not get ID for {}", path.display()))?
        .into())
}

// Read a newick file and extract the identifier
pub fn read_tree(treepath: &Path) -> Result<(String, Tree)> {
    let mut tree = Tree::from_file(treepath).context(format!(
        "Could not parse newick file: {}",
        treepath.display()
    ))?;

    tree.reset_depths()?;

    Ok((get_file_id(treepath)?, tree))
}

// Load reference trees
pub fn read_refs(ref_dir: &Path) -> Result<HashMap<String, Tree>> {
    let trees: Result<Vec<_>> = trees_iter(ref_dir)?.collect();
    Ok(HashMap::from_iter(trees?))
}

// Iterate over newick files in a directory and parse them
pub fn trees_iter(dir: &Path) -> Result<impl Iterator<Item = Result<(String, Tree)>>> {
    Ok(fs::read_dir(dir)?
        .flatten()
        .map(|e| e.path())
        .filter(|p| is_newick(p))
        .map(|p| read_tree(&p)))
}

// Add .gz extension to filepath if needed
pub fn add_gz_ext(path: PathBuf) -> PathBuf {
    match path.extension().and_then(OsStr::to_str) {
        Some("gz") => path,
        _ => {
            let mut path_str: OsString = path.into_os_string();
            path_str.push(".gz");
            path_str.into()
        }
    }
}

// Init uncompressed writer
pub fn init_writer(path: PathBuf) -> Result<impl std::io::Write> {
    let output = File::create(path).context("Could not create output file.")?;
    let writer = BufWriter::new(output);

    Ok(writer)
}

// Init gzipped writer
pub fn init_gz_writer(path: PathBuf) -> Result<impl std::io::Write> {
    let output_path = add_gz_ext(path);
    let output = File::create(output_path).context("Could not create compressed output file")?;

    let writer = SyncZBuilder::<Gzip, _>::new().from_writer(output);

    Ok(writer)
}

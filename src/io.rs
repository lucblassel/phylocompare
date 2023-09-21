use anyhow::{Context, Result};
use phylotree::tree::Tree;
use std::{collections::HashMap, ffi::OsStr, fs, path::Path};

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
    let tree = Tree::from_file(treepath).context(format!(
        "Could not parse newick file: {}",
        treepath.display()
    ))?;

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

use anyhow::{bail, Context, Result};
use flate2::{write::GzEncoder, Compression};
use phylotree::tree::Tree;
use std::{
    collections::HashMap,
    ffi::{OsStr, OsString},
    fs::{self, metadata, File},
    io::{self},
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

// Initialize write with out without compression
pub fn init_writer(path: PathBuf, zipped: bool) -> Result<Box<dyn io::Write + 'static>> {
    let file = File::create(&path).context("Could not create output file")?;
    Ok(if zipped {
        Box::new(GzEncoder::new(file, Compression::default()))
    } else {
        Box::new(file)
    })
}

// Create CSV wrriter from IO writer
pub fn from_writer<W: io::Write>(wtr: W) -> csv::Writer<W> {
    csv::Writer::from_writer(wtr)
}

// Get output writer, zipped or not
pub fn get_output(
    path: PathBuf,
    zipped: bool,
    is_some: bool,
) -> Result<Option<csv::Writer<Box<dyn io::Write>>>> {
    Ok(if is_some {
        Some(from_writer(init_writer(path, zipped)?))
    } else {
        None
    })
}

pub fn get_suffixed_filenme(path: &PathBuf, suffix: &str, ext: &str, zip: bool) -> Result<PathBuf> {
    let mut pb = path.clone();
    let mut stem = pb.clone();
    let mut previous_stem = stem.clone();

    let mut guard = 0;
    while let Some(new_stem) = stem.file_stem() {
        if guard > 100 {
            bail! {"Could not deduce file prefix for: {}", pb.display()}
        }
        stem = PathBuf::from(new_stem);
        if stem == previous_stem {
            break;
        }
        previous_stem = stem.clone();
        guard += 1;
    }

    let stem_str = stem
        .to_str()
        .context("Could not convert output file name to string")?;

    pb.set_file_name(format!("{stem_str}_{suffix}"));
    pb.set_extension(ext);

    Ok(if zip { add_gz_ext(pb) } else { pb })
}

use std::collections::BTreeMap;

use anyhow::{anyhow, Result};
use itertools::Itertools;
use phylotree::tree::Comparison;

const TREES_HEADER: [&str; 6] = ["id", "size", "rf", "norm_rf", "rf_weight", "kf_score"];
const DISTS_HEADER: [&str; 3] = ["id", "ref", "comp"]; // Used for pairwise distances and branch
                                                       // length outputs

pub enum CSVType {
    Trees,
    Distances,
    Branches,
}

// Get Header for csv output files
pub fn get_header_string(csv_type: CSVType) -> String {
    match csv_type {
        CSVType::Trees => TREES_HEADER.join(","),
        CSVType::Distances | CSVType::Branches => DISTS_HEADER.join(","),
    }
}

fn add_marker(csv: &mut String, markers: &str) {
    csv.push(',');
    csv.push_str(markers)
}

// Format tree comparison as a csv record
pub fn format_tree_record(
    id: &str,
    size: usize,
    cmp: &Comparison,
    markers: Option<&str>,
) -> String {
    let mut csv = format!(
        "{id},{size},{},{},{},{}",
        cmp.rf, cmp.norm_rf, cmp.weighted_rf, cmp.branch_score
    );

    if let Some(markers) = markers {
        add_marker(&mut csv, markers)
    }

    csv
}

// Format branch length comparison as a csv record
pub fn format_branch_record(
    id: &str,
    reflen: Option<f64>,
    cmplen: Option<f64>,
    markers: Option<&str>,
) -> String {
    let ref_s = reflen.map(|v| format!("{v}")).unwrap_or("".into());
    let cmp_s = cmplen.map(|v| format!("{v}")).unwrap_or("".into());

    let mut csv = format!("{id},{ref_s},{cmp_s}");
    if let Some(markers) = markers {
        add_marker(&mut csv, markers)
    }
    csv
}

// Parse JSON k-v store to CSV headers and values
pub fn parse_markers(json: &str) -> Result<(String, String)> {
    let lookup: BTreeMap<String, String> = serde_json::from_str(json)?;
    let keys: Vec<String> = lookup.keys().sorted().map(String::from).collect();

    let header = keys.iter().join(",");
    let values = keys
        .iter()
        .map(|k| lookup.get(k).ok_or(anyhow!("Key `{k}` not found")))
        .collect::<Result<Vec<_>>>()?
        .iter()
        .join(",");

    Ok((header, values))
}

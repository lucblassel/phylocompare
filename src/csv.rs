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

// Format tree comparison as a csv record
pub fn format_tree_record(id: &str, size: usize, cmp: &Comparison) -> String {
    format!(
        "{id},{size},{},{},{},{}",
        cmp.rf, cmp.norm_rf, cmp.weighted_rf, cmp.branch_score
    )
}

// Format branch length comparison as a csv record
pub fn format_branch_record(id: &str, reflen: Option<f64>, cmplen: Option<f64>) -> String {
    let ref_s = reflen.map(|v| format!("{v}")).unwrap_or("".into());
    let cmp_s = cmplen.map(|v| format!("{v}")).unwrap_or("".into());

    format!("{id},{ref_s},{cmp_s}")
}

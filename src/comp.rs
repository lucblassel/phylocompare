use anyhow::Result;
use itertools::Itertools;
use phylotree::tree::{Comparison, Tree};
use serde::Serialize;

#[derive(Serialize, Default, Debug)]
pub struct BranchRecord {
    pub id: String,
    pub ref_len: Option<f64>,
    pub ref_depth: Option<usize>,
    pub cmp_len: Option<f64>,
    pub cmp_depth: Option<usize>,
    pub marker: Option<String>,
}

impl BranchRecord {
    fn from_trees(
        reftree: &Tree,
        cmptree: &Tree,
        include_tips: bool,
        id: &String,
    ) -> Result<Vec<Self>> {
        let (reference, compared, common) =
            reftree.compare_branch_lengths(cmptree, include_tips)?;
        let mut records = Vec::new();

        records.extend(reference.into_iter().map(|(d, l)| BranchRecord {
            id: id.clone(),
            ref_len: Some(l),
            ref_depth: Some(d),
            ..Default::default()
        }));

        records.extend(compared.into_iter().map(|(d, l)| BranchRecord {
            id: id.clone(),
            cmp_len: Some(l),
            cmp_depth: Some(d),
            ..Default::default()
        }));

        records.extend(common.into_iter().map(|((rd, rl), (cd, cl))| BranchRecord {
            id: id.clone(),
            ref_depth: Some(rd),
            ref_len: Some(rl),
            cmp_len: Some(cl),
            cmp_depth: Some(cd),
            ..Default::default()
        }));

        Ok(records)
    }
}

#[derive(Default, Debug, Serialize)]
pub struct DistanceRecord {
    pub id: String,
    pub ref_dist: f64,
    pub cmp_dist: f64,
    pub marker: Option<String>,
}

impl DistanceRecord {
    fn get_cap(size: usize) -> usize {
        size * (size - 1) / 2
    }

    fn from_trees(reftree: &Tree, cmptree: &Tree, id: &String) -> Result<Vec<Self>> {
        let mut dists = Vec::with_capacity(Self::get_cap(reftree.n_leaves()));
        let ref_dists = reftree.distance_matrix()?;
        let cmp_dists = cmptree.distance_matrix()?;

        for pair in ref_dists.taxa.iter().combinations(2) {
            let (tip_1, tip_2) = (pair[0], pair[1]);

            let &ref_dist = ref_dists.get(tip_1, tip_2).unwrap_or(&f64::NAN);
            let &cmp_dist = cmp_dists.get(tip_1, tip_2).unwrap_or(&f64::NAN);

            dists.push(Self {
                id: id.clone(),
                ref_dist,
                cmp_dist,
                ..Default::default()
            });
        }

        Ok(dists)
    }
}

#[derive(Debug, Default, Serialize)]
pub struct TopologyRecord {
    pub id: String,
    pub rf: f64,
    pub norm_rf: f64,
    pub weighted_rf: f64,
    pub kf_score: f64,
    pub n_tips: usize,
    pub marker: Option<String>,
}

impl From<Comparison> for TopologyRecord {
    fn from(value: Comparison) -> Self {
        Self {
            rf: value.rf,
            norm_rf: value.norm_rf,
            weighted_rf: value.weighted_rf,
            kf_score: value.branch_score,
            ..Default::default()
        }
    }
}

#[derive(Default, Debug)]
pub struct ComparisonRecord {
    pub topology: Option<TopologyRecord>,
    pub branches: Option<Vec<BranchRecord>>,
    pub distances: Option<Vec<DistanceRecord>>,
}

pub fn compare_trees(
    id: impl Into<String>,
    reftree: &Tree,
    cmptree: &Tree,
    compare_topo: bool,
    compare_lens: bool,
    compare_dist: bool,
    include_tips: bool,
) -> Result<ComparisonRecord> {
    let mut record = ComparisonRecord {
        topology: None,
        branches: None,
        distances: None,
    };

    let id = id.into();

    // Compare topologies
    if compare_topo {
        let mut topo = TopologyRecord::from(reftree.compare_topologies(cmptree)?);
        topo.n_tips = reftree.n_leaves();
        topo.id = id.clone();
        record.topology = Some(topo);
    }

    // Compare edges
    if compare_lens {
        record.branches = Some(BranchRecord::from_trees(
            reftree,
            cmptree,
            include_tips,
            &id,
        )?);
    }

    // Compare distances
    if compare_dist {
        record.distances = Some(DistanceRecord::from_trees(reftree, cmptree, &id)?);
    }

    Ok(record)
}

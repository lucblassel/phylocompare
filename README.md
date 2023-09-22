# `PhyloCompare`

This is a tool to compare phylogenetic trees in different directories, to trees contained in a reference directory. 
The trees are matched according to filename.  
This tool was made with the goal of benchmarking several different tree reconstruction methods to a set of "real" simulated trees. 

## Functions
This tool can output several different pieces of information:
- Topological distance metrics such as Robinson-Foulds or Khuner-Felsenstein distances
- Comparison of extracted pairwise distances
- Branches in common or exclusive to each tree and their lengths

## Benchmarking
`hyperfine --export-markdown bench.md --warmup 5 './target/release/phylocompare  -o t.gz ../get_rf/test/tree
s1 ../get_rf/test/trees.renamed'`

- sequential: `..._seq` (2a7bb24b47fa9b9b1a39e5ed81369cbb1cc440ac)
- Rayon: `..._rayon` (87b800691228e358716dff9f5fb24e32d10d35ea)
- Rayon+crossbeam: `..._rcb` (3736ff9484e5968e11640e15c459b8725b72d341)

| Command | Mean [s] | Min [s] | Max [s] | Relative |
|:---|---:|---:|---:|---:|
| `./target/release/phylocompare_seq  -o t.gz ../get_rf/test/trees1 ../get_rf/test/trees.renamed` | 1.839 ± 0.062 | 1.783 | 1.935 | 2.71 ± 0.10 |
| `./target/release/phylocompare_rayon -o tr.gz ../get_rf/test/trees1 ../get_rf/test/trees.renamed/` | 0.710 ± 0.042 | 0.681 | 0.807 | 1.05 ± 0.06 |
| `./target/release/phylocompare_rcb -o tcb.gz ../get_rf/test/trees1 ../get_rf/test/trees.renamed/` | 0.678 ± 0.010 | 0.665 | 0.693 | 1.00 |


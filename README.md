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

### Not parallelized version (2a7bb24b47fa9b9b1a39e5ed81369cbb1cc440ac)
| Command | Mean [s] | Min [s] | Max [s] | Relative |
|:---|---:|---:|---:|---:|
| `./target/release/phylocompare  -o t.gz ../get_rf/test/trees1 ../get_rf/test/trees.renamed` | 1.776 ± 0.025 | 1.754 | 1.823 | 1.00 |

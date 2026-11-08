# Upstream reference tests: NOT RUN

These are complete OpenMed test files (maziyarpanahi/openmed@3e232328), renamed to
the `medscale_clinical` namespace and otherwise unchanged. They exercise
upstream modules that this package does not ship (the SDOH extractor and
section parser, the NLI verifier, persistence-linked review transitions), so
pytest does not collect them (`.py.txt`) and **none of them is claimed to
pass**. The self-contained tests from the same files run from `tests/`.

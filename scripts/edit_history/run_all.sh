#!/usr/bin/env bash

echo "Downloading dumps..."
sh download_dumps.sh
echo "Dumps downloaded"

cd wd_diff_calculator
cargo build --release
cargo run --release -- --input-dir ../../../data/edit_history/raw_dumps --output-dir ../../../data/edit_history/diffs --bulk-size 100 --entities-file ../../../data/edit_history/entities_ids.txt

cd ../elasticsearch_indexer
cargo build --release
cargo run --release -- --input-dir ../../../data/edit_history/diffs --bulk-size 100

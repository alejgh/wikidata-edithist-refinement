#!/usr/bin/env bash

# this line is needed when running from the docker-compose file
# it can be deleted if the script is run from this folder
cd scripts/edit_history

echo "Downloading dumps..."
#sh download_dumps.sh
echo "Dumps downloaded"

cd wd_diff_calculator
cargo build --release
cargo run --release -- --input-dir ../../../data/edit_history/raw_dumps --output-dir ../../../data/edit_history/diffs --bulk-size 100 --entities-file ../../../data/edit_history/entities_ids.txt > progress.txt

cd ../diff_indexer
cargo build --release
#cargo run --release -- --input-dir ../../../data/edit_history/diffs --bulk-size 15

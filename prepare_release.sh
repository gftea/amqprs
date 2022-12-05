#!/bin/bash

#// quick local check before release

release_tag=$1

Usage() { echo "missing release tag"; exit 1; }
[ $# -ne 1 ] && Usage

cargo test 
cargo test -F tracing
cargo test -F compliance_assert
cargo test --all-features 

cargo doc  -p amqprs --all-features --open
cargo publish -p amqprs --all-features --dry-run
 

git commit -a -m "prepare release ${release_tag}"
git tag -a ${release_tag} -m "${release_tag}"
git push origin ${release_tag}
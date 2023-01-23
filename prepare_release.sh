#!/bin/bash
#// prepare release

function check_result () {
    ret=$?
    if [[ $ret != 0 ]];then
        echo "early exit due to error!"
        exit $ret
    fi
}

version=$(egrep '^version = "(.+)"$' -o amqprs/Cargo.toml | cut -d '"' -f2)

# check semver
semver_regex="[0-9]+\.[0-9]+\.[0-9]+"
if ! [[ $version =~ $semver_regex ]]; then
    echo "error, check semantic version: '$version'"
    exit 1  
fi

# dry run check
cargo publish -p amqprs --all-features --dry-run
check_result

cargo package --list
check_result

ls -hl target/package/amqprs-${version}.crate
check_result

read -p "Are you going to release v${version}? " ans
if [ "$ans" != "y" ]; then
    exit 0
fi

read -p 'make a commit? ' ans
if [ "$ans" = "y" ]; then
    git commit -a -m "prepare release v${version}"
    git log -1
fi

read -p 'push commit? ' ans
if [ "$ans" = "y" ]; then
    git push
fi

read -p "push tag v${version}? " ans
if [ "$ans" = "y" ]; then
    git tag -a "v${version}" -m "v${version}"
    git push origin "v${version}"
    git log -1
fi

read -p 'Want to publish to crates.io? ' ans
if [ "$ans" = "y" ]; then
    cargo publish -p amqprs --all-features
fi

#!/bin/bash
#// prepare release

version=$(egrep '^version = "(.+)"$' -o amqprs/Cargo.toml | cut -d '"' -f2)

# check 
semver_regex="[0-9]+\.[0-9]+\.[0-9]+"
if ! [[ $version =~ $semver_regex ]]; then
    echo "error, check semantic version: '$version'"
    exit 1  
fi

read -p "Are you going to release $version? " ans
if [ "$ans" != "y" ]; then
    exit 0
fi

read -p 'commit? ' ans
if [ "$ans" = "y" ]; then
    git commit -a -m "prepare release ${version}"
    git tag -a ${version} -m "${version}"
    git log -1
fi
read -p 'push tag? ' ans
if [ "$ans" = "y" ]; then
    git push origin ${version}
fi
read -p 'push commit? ' ans
if [ "$ans" = "y" ]; then
    git push
fi

read -p 'Want to publish to crates.io? ' ans
if [ "$ans" = "y" ]; then
    cargo publish -p amqprs --all-features
fi

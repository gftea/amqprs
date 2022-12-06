#!/bin/bash
#// prepare release

release_tag=$1

Usage() { echo "missing release tag"; exit 1; }
[ $# -ne 1 ] && Usage

read -p 'commit: ' ans
if [ "$ans" = "y" ]; then
    git commit -a -m "prepare release ${release_tag}"
    git tag -a ${release_tag} -m "${release_tag}"
    git log -1
fi
read -p 'push tag: ' ans
if [ "$ans" = "y" ]; then
    git push origin ${release_tag}
fi
read -p 'push commit: ' ans
if [ "$ans" = "y" ]; then
    git push
fi
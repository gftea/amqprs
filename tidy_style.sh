#!/bin/bash



function trim_trailing_whitespaces () {
    src_file=$1
    awk -i inplace '{ sub(/[ \t]+$/, ""); print }' $src_file
}


target_files=$(find . -type f -name "*.rs" -not -path '*/target/*')

for src_file in ${target_files[@]}; do
    trim_trailing_whitespaces $src_file   
done

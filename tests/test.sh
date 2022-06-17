#!/bin/bash

cd $(pwd)/$(dirname $0)/..
cargo test

which avc2 > /dev/null
has_avc=$?

test() {
    echo testing assembly for $1
    cargo run -q -- tests/$1.avc tests/$1_test.avcr || { echo $1 failed to compile!; return; }
    cmp tests/$1.avcr tests/$1_test.avcr || { echo $1 compiled differently!; rm tests/$1_test.avcr; return; }
    if [ $has_avc -eq 0 ]; then
        avc2 tests/$1_test.avcr
    fi
    rm tests/$1_test.avcr
}

test h
test rel
test mactest

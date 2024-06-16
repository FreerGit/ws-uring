#!/bin/bash

for test in ./bin/test_*;
do
    if [ -x "$test" ]; then
        ./"$test"
    else
        echo "Skipping $test, not an executable?"
    fi
done
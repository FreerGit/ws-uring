#!/bin/bash

for test in ./bin/test_*;
do
    if [ -x "$test" ]; then
        ./"$test"
        if [ $? -ne 0 ]
        then
            echo "Stopping tests..."
            break
        fi
    else
        echo "Skipping $test, not an executable?"
    fi
done
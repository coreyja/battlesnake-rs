#!/usr/bin/env bash

declare -i NUMBER_OF_RUNS=10

sumss=0
for i in $(seq $NUMBER_OF_RUNS)  # you can also use {0..9}
do
  declare -i thisRun
  thisRun=$(battlesnake play -g solo -n 'test' -u "http://localhost:8000" -H 7 -W 7 |& grep 'DONE' | sed -n "s/.*Game completed after \(.*\) turn.*/\1/p")
  echo "Run $i: $thisRun"
  sumss=$((sumss+thisRun))
done

echo "Average: $((sumss/NUMBER_OF_RUNS))"


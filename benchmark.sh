#!/bin/bash
# Example: ./benchmark.sh 10 2 target/release/psync /path/to/dir

function empty_cache() {
    sync
    echo 3 > /proc/sys/vm/drop_caches
}

times=$1
threads=$2
psync=$3
src=$4
dst=/tmp/psync

# Need sudo for empty_cache, so ask for password once upfront
sudo whoami >/dev/null

for cmd in "cp -R $src $dst" "$psync -t $threads $src $dst"; do
    echo "Benchmarking: $cmd"
    elapsed=0
    for i in $(seq $times); do
        rm -rf $dst
        sudo bash -c "$(declare -f empty_cache); empty_cache"
        ts=$(date +%s%N)
        eval $cmd > /dev/null
        if [ $? -ne 0 ]; then
            echo "Command failed"
            exit 1
        fi
        elapsed=$((elapsed + $(date +%s%N) - $ts))
    done
    echo "Average time: $(($elapsed/1000000/$times))ms"
done

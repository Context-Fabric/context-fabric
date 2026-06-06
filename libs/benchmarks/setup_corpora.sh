#!/bin/sh
set -eu

ROOT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
CORPORA_DIR="$ROOT_DIR/libs/benchmarks/.corpora"

BHSA_SHA="4db00e2157915495e1a4d3d57e41223df24775da"
N1904_SHA="def0fd230b9ab77c11c371b273d020b1a573c884"
BANKS_SHA="cf84038eb81df3d8f3aed14ec48ec7739871e463"

setup_corpus() {
    name=$1
    repo=$2
    sparse_path=$3
    sha=$4

    target="$CORPORA_DIR/$name/tf"
    marker="$CORPORA_DIR/$name/.setup-done"

    if [ -f "$target/otype.tf" ] && [ -f "$marker" ] && [ "$(cat "$marker")" = "$sha" ]; then
        echo "$name already provisioned at $target ($sha)"
    else
        tmp="$CORPORA_DIR/.tmp-$name"
        rm -rf "$tmp" "$target"
        mkdir -p "$CORPORA_DIR/$name"

        git clone --filter=blob:none --no-checkout "$repo" "$tmp"
        (
            cd "$tmp"
            git sparse-checkout init --no-cone
            git sparse-checkout set "$sparse_path"
            git checkout "$sha"
        )

        mkdir -p "$target"
        cp -R "$tmp/$sparse_path/." "$target/"
        rm -rf "$tmp"
        printf '%s' "$sha" > "$marker"
    fi

    for required in otype.tf oslots.tf otext.tf; do
        if [ ! -f "$target/$required" ]; then
            echo "missing required file: $target/$required" >&2
            exit 1
        fi
    done

    if [ "$name" = "bhsa" ]; then
        for required in book@en.tf book@he.tf; do
            if [ ! -f "$target/$required" ]; then
                echo "missing required BHSA multilingual file: $target/$required" >&2
                exit 1
            fi
        done
    fi

    if [ "$name" = "banks" ] && ! grep -q '^@structureTypes=' "$target/otext.tf"; then
        echo "missing required banks structureTypes metadata: $target/otext.tf" >&2
        exit 1
    fi

    file_count=$(find "$target" -type f | wc -l | tr -d ' ')
    byte_count=$(find "$target" -type f -exec wc -c {} + | awk 'END { print $1 }')
    echo "$name ready: $file_count files, $byte_count bytes"
}

mkdir -p "$CORPORA_DIR"
setup_corpus "bhsa" "https://github.com/ETCBC/bhsa" "tf/2021" "$BHSA_SHA"
setup_corpus "n1904" "https://github.com/CenterBLC/N1904" "tf/1.0.0" "$N1904_SHA"
setup_corpus "banks" "https://github.com/annotation/banks" "tf/0.2" "$BANKS_SHA"

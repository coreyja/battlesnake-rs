#!/usr/bin/env bash

set -e

SCRIPT=$(realpath "$0")
SCRIPTPATH=$(dirname "$SCRIPT")
REPO_ROOT_PATH="$SCRIPTPATH/../.."

if [ "$1" == "release" ]; then
  BUILD_ARGS="--release"
  TARGET_DIR="release"
else
  TARGET_DIR="debug"
fi

function musl-build() {
  pushd "$REPO_ROOT_PATH"
    # docker run \
    #   -v cargo-git:/home/rust/.cargo/git \
    #   -v cargo-registry:/home/rust/.cargo/registry \
    #   -v "$PWD":/home/rust/src \
    #   --rm -it ekidd/rust-musl-builder:nightly cargo build -p web-lambda $BUILD_ARGS
    cargo build --target x86_64-unknown-linux-musl -p web-lambda
  popd
}

function lambdaify() {
  TARGET="$1"

  TMP_DIR="$(mktemp -d)"
  cp "$REPO_ROOT_PATH/target/x86_64-unknown-linux-musl/$TARGET_DIR/$TARGET" "$TMP_DIR/bootstrap"

  mkdir -p "$REPO_ROOT_PATH/target/lambda/"
  zip -j "$REPO_ROOT_PATH/target/lambda/$TARGET.zip" "$TMP_DIR/bootstrap"
  rm -r "$TMP_DIR"
}

musl-build

lambdaify "web-lambda"

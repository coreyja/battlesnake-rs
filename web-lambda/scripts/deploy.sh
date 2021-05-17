#!/usr/bin/env bash

set -e

SCRIPT=$(realpath "$0")
SCRIPTPATH=$(dirname "$SCRIPT")
REPO_ROOT_PATH="$SCRIPTPATH/../.."

pushd "$REPO_ROOT_PATH/web-lambda" > /dev/null
  ./scripts/build.sh release

  sam deploy
popd > /dev/null

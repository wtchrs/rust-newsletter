#!/usr/bin/env bash

set -x
set -eo pipefail

if ! [ -x "$(command -v bunyan)" ]; then
  echo >&2 "Error: bunyan is not installed."
  echo >&2 "Install by running: \"cargo install bunyan\" or \"cargo binstall bunyan\""
  exit 1
fi

cargo run | bunyan

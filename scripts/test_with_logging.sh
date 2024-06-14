#!/usr/bin/env bash

export RUST_LOG="sqlx=error,info"
export TEST_LOG="enabled"
cargo test "$@" | bunyan

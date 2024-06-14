#!/usr/bin/env bash

export RUST_LOG="sqlx=error,info"
export TEST_LOG="enabled"
cargo test subscribe_fails_if_there_is_a_fatal_database_error | bunyan

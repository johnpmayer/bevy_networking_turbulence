#!/usr/bin/env bash

RUST_LOG=debug cargo run --example simple --no-default-features --features use-webrtc -- --server

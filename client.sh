#!/usr/bin/env bash

cargo build --example simple --target wasm32-unknown-unknown --no-default-features --features use-webrtc

wasm-bindgen --out-dir target --target web target/wasm32-unknown-unknown/debug/examples/simple.wasm

basic-http-server .

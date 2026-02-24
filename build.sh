#!/usr/bin/env bash
set -e

mkdir -p dist
rm -rf dist/pkg dist/js

wasm-pack build --target web --out-dir dist/pkg

mkdir -p dist/js
cp -r static/* dist/
cp -r js/* dist/js/

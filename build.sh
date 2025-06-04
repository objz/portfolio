#!/usr/bin/env bash
set -e

rm -rf dist
mkdir -p dist/js

wasm-pack build --target web --out-dir dist/pkg

cp -r static/* dist/
cp -r js/* dist/js/

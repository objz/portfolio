#!/bin/zsh
wasm-pack build --target web --out-dir pkg
rm -rf dist 
mkdir -p dist/js
cp -r static/* dist/
cp -r js/* dist/js/
cp -r pkg dist/

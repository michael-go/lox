#!/bin/bash

set -e
set -x

cd "$(dirname "$0")"

ORIGIN_COMMIT_SHA=01e6f5b8f3e5dfa65674c2f9cf4700d73ab41cf8
wget -N https://github.com/munificent/craftinginterpreters/archive/${ORIGIN_COMMIT_SHA}.zip -O craftinginterpreters.zip

rm -rf craftinginterpreters/
unzip craftinginterpreters.zip
mv craftinginterpreters-${ORIGIN_COMMIT_SHA} craftinginterpreters

cd craftinginterpreters
make get

dart tool/bin/test.dart jlox -i ../../../golox/golox
dart tool/bin/test.dart clox -i ../../../rslox/target/release/rslox
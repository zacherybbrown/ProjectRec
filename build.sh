#!/usr/bin/env bash
set -e

cd "$(dirname "$0")"
echo "Building Project Rec in release mode..."
if ! cargo build --release; then
    echo "Build failed."
    exit 1
fi

OUTPUT_DIR="build"

rm -rf "$OUTPUT_DIR"
mkdir -p "$OUTPUT_DIR"
echo "Copying executable..."
if ! cp "target/release/project_rec" "$OUTPUT_DIR/project_rec"; then
    echo "Failed to copy executable."
    exit 1
fi

echo "Copying assets..."
if ! cp -r "assets" "$OUTPUT_DIR/assets"; then
    echo "Failed to copy assets."
    exit 1
fi

echo "Build output ready in $OUTPUT_DIR/"

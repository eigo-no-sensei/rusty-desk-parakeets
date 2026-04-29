#!/bin/bash
# Build script for Parakeet TDT App
# Builds the frontend and Tauri backend

set -e

echo "=== Building Parakeet TDT App ==="

# Check for required tools
command -v npm >/dev/null 2>&1 || { echo "npm required but not found. Aborting." >&2; exit 1; }
command -v cargo >/dev/null 2>&1 || { echo "cargo required but not found. Aborting." >&2; exit 1; }

# Build frontend
echo ""
echo "=== Building Frontend ==="
npm install
npm run build

# Build Tauri
echo ""
echo "=== Building Tauri Backend ==="
cd src-tauri
cargo build --release

echo ""
echo "=== Build Complete ==="
echo "Frontend: src/"
echo "Backend:  src-tauri/target/release/parakeet-tdt-app"

# Show executable info
if [ -f "src-tauri/target/release/parakeet-tdt-app" ]; then
    ls -lh src-tauri/target/release/parakeet-tdt-app
fi
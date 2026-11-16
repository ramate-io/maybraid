#!/usr/bin/env bash
set -euo pipefail

# Capture headless frames of generated boss via HCSG presentation (#1116)
# Uses xvfb-run with Lavapipe for headless rendering

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../../../.." && pwd)"
OUTPUT_DIR="$SCRIPT_DIR"

cd "$PROJECT_ROOT"

echo "Building maybraid-world with debug-commands..."
PKG_CONFIG_PATH=/usr/lib/x86_64-linux-gnu/pkgconfig \
  CXX=g++-13 CC=gcc-13 LIBRARY_PATH=/usr/lib/gcc/x86_64-linux-gnu/13 \
  cargo build --release -p maybraid-world-playground --features maybraid-world/debug-commands

echo "Capturing boss generation frames with Lavapipe..."

# Run world with boss spawn command and capture screenshot
VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json \
  WGPU_BACKEND=vk \
  xvfb-run -a -s "-screen 0 1280x720x24" \
  "$PROJECT_ROOT/target/release/maybraid-world" \
  --start-at 0,0 \
  "screenshot $OUTPUT_DIR/boss-at-origin.png" \
  "wait 120" \
  "boss spawn" \
  "wait 120" \
  "screenshot $OUTPUT_DIR/boss-spawned.png" \
  "wait 30" \
  "exit" \
  2>&1 | tee "$OUTPUT_DIR/capture.log"

echo "Frames captured:"
ls -lh "$OUTPUT_DIR"/*.png || echo "No PNG files generated"

#!/usr/bin/env bash
# Full correctness pass over the engine the page ships.
set -e
cd "$(dirname "$0")/.."
python3 tools/extract-engine.py
cd tools/tests
echo "── move generation (perft) ──"; node test.mjs
echo "── game data (PGN + study FENs) ──"; node pgn.mjs
echo "── static exchange evaluation ──"; node seetest.mjs

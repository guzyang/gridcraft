#!/usr/bin/env bash
# Publish the next GridCraft build (v6): references into other workbooks are followed, and a
# circular reference reads 0 as Excel shows it.
#
# Run it from the repo root after the push to the build repo has been approved:
#   bash C:/Users/david/AppData/Local/hermes/cache/scratch/gc2/scripts/publish-v6.sh
set -euo pipefail

REPO="C:/Users/david/AppData/Local/hermes/cache/scratch/gc2"
GH="C:/Users/david/tools/bin/gh.exe"
BUILD_REPO="guzyang/gridcraft-mac-build"
VERSION="v0.19.1-v6"
DMG_DIR="C:/Users/david/AppData/Local/hermes/cache/scratch/dmg-v6"

cd "$REPO"

echo "== 1. push the code to the build repo =="
git push -f build build-only:main

echo "== 2. start the Apple Silicon build =="
"$GH" workflow run macos-unsigned.yml --repo "$BUILD_REPO" --ref main

echo "== 3. wait for the build =="
RUN=""
for _ in $(seq 1 80); do
  sleep 30
  RUN=$("$GH" run list --repo "$BUILD_REPO" --limit 1 --json databaseId -q '.[0].databaseId')
  STATE=$("$GH" run view "$RUN" --repo "$BUILD_REPO" --json status,conclusion -q '.status + " " + (.conclusion // "")')
  echo "   run $RUN: $STATE"
  case "$STATE" in
    completed*) break ;;
  esac
done

echo "== 4. download the .dmg =="
rm -rf "$DMG_DIR"
"$GH" run download "$RUN" --repo "$BUILD_REPO" --name gridcraft-macos-unsigned --dir "$DMG_DIR"
cd "$DMG_DIR"
mv gridcraft-0.19.1-macos-aarch64.dmg "gridcraft-0.19.1-v6-macos-aarch64.dmg"

echo "== 5. publish the release =="
"$GH" release create "$VERSION" --repo "$BUILD_REPO" \
  --title "GridCraft 0.19.1 v6：外部引用真同步 + 循环引用按 Excel 显示" \
  --notes-file "$REPO/docs/release-v6.md" \
  "gridcraft-0.19.1-v6-macos-aarch64.dmg#gridcraft-0.19.1-v6-macos-aarch64.dmg"

echo "== done =="
"$GH" release view "$VERSION" --repo "$BUILD_REPO" --json url -q .url

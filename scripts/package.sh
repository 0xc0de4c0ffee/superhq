#!/bin/bash
set -euo pipefail

PROJECT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
APP_NAME="SuperHQ"
BUNDLE_ID="com.superhq.app"
VERSION="${VERSION:-$(grep '^version' "$PROJECT_DIR/Cargo.toml" | head -1 | sed 's/.*"\(.*\)"/\1/')}"

BUILD_DIR="$PROJECT_DIR/target/release"
BUNDLE_DIR="$PROJECT_DIR/target/package"
APP_BUNDLE="$BUNDLE_DIR/$APP_NAME.app"
DMG_PATH="$PROJECT_DIR/target/$APP_NAME-$VERSION.dmg"

echo "=== SuperHQ Packaging ==="
echo ""

# --- Step 1: Generate icon ---
echo "[1/5] Generating app icon..."
python3 "$PROJECT_DIR/scripts/generate_icon.py"

# --- Step 2: Build release binary ---
echo ""
echo "[2/5] Building release binary..."
cargo build --release --manifest-path "$PROJECT_DIR/Cargo.toml"

# --- Step 3: Create .app bundle ---
echo ""
echo "[3/5] Creating app bundle..."
rm -rf "$APP_BUNDLE"
mkdir -p "$APP_BUNDLE/Contents/MacOS"
mkdir -p "$APP_BUNDLE/Contents/Resources"

# Copy binary
cp "$BUILD_DIR/superhq" "$APP_BUNDLE/Contents/MacOS/superhq"

# Copy Info.plist
cp "$PROJECT_DIR/Info.plist" "$APP_BUNDLE/Contents/Info.plist"

# Copy icon
cp "$PROJECT_DIR/assets/AppIcon.icns" "$APP_BUNDLE/Contents/Resources/AppIcon.icns"

# Write PkgInfo
echo -n "APPL????" > "$APP_BUNDLE/Contents/PkgInfo"

echo "  Bundle created: $APP_BUNDLE"

# --- Step 4: Code sign ---
echo ""
echo "[4/5] Code signing..."
codesign --sign - \
    --entitlements "$PROJECT_DIR/entitlements.plist" \
    --force \
    --deep \
    "$APP_BUNDLE"

echo "  Signed with ad-hoc identity"

# --- Step 5: Create DMG ---
echo ""
echo "[5/5] Creating DMG..."
rm -f "$DMG_PATH"

# Create a temporary DMG directory with the app and an Applications symlink
DMG_STAGING="$BUNDLE_DIR/dmg-staging"
rm -rf "$DMG_STAGING"
mkdir -p "$DMG_STAGING"
cp -R "$APP_BUNDLE" "$DMG_STAGING/"
ln -s /Applications "$DMG_STAGING/Applications"

hdiutil create \
    -volname "$APP_NAME" \
    -srcfolder "$DMG_STAGING" \
    -ov \
    -format UDZO \
    "$DMG_PATH"

rm -rf "$DMG_STAGING"

echo ""
echo "=== Done ==="
echo "  App: $APP_BUNDLE"
echo "  DMG: $DMG_PATH"

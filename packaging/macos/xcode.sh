#!/usr/bin/env bash
# Selected Xcode for macOS release builds. Not "whatever is on PATH".
# Override with MAYBRAID_XCODE=/Applications/Xcode_16.2.app
#
# Requires Xcode 15+ (Metal + macOS 13 SDK). CI uses macos-14's Xcode.app.
# This is an explicit external toolchain, not a reproducible Nix compiler.

MAYBRAID_XCODE="${MAYBRAID_XCODE:-/Applications/Xcode.app}"
MAYBRAID_MACOSX_DEPLOYMENT_TARGET="${MAYBRAID_MACOSX_DEPLOYMENT_TARGET:-13.0}"

if [[ ! -d "$MAYBRAID_XCODE/Contents/Developer" ]]; then
	echo "xcode: $MAYBRAID_XCODE is not an Xcode app" >&2
	exit 1
fi

export DEVELOPER_DIR="$MAYBRAID_XCODE/Contents/Developer"
export SDKROOT="$(xcrun --sdk macosx --show-sdk-path)"
export CC="$(xcrun --find clang)"
export CXX="$(xcrun --find clang++)"
export AR="$(xcrun --find ar)"
export MACOSX_DEPLOYMENT_TARGET="$MAYBRAID_MACOSX_DEPLOYMENT_TARGET"
export CFLAGS="-isysroot $SDKROOT -mmacosx-version-min=$MACOSX_DEPLOYMENT_TARGET"
export CXXFLAGS="-isysroot $SDKROOT -stdlib=libc++ -mmacosx-version-min=$MACOSX_DEPLOYMENT_TARGET"
export LDFLAGS="-isysroot $SDKROOT -mmacosx-version-min=$MACOSX_DEPLOYMENT_TARGET"
# Apple libiconv only. Do not add Nix/Homebrew search paths.
export RUSTFLAGS="-L native=$SDKROOT/usr/lib -L native=/usr/lib"

unset NIX_LDFLAGS NIX_CFLAGS_COMPILE NIX_CC LIBRARY_PATH CPATH \
	C_INCLUDE_PATH CPLUS_INCLUDE_PATH PKG_CONFIG_PATH || true

xcode_ver="$(xcodebuild -version 2>/dev/null | awk '/Xcode/{print $2; exit}')"
xcode_major="${xcode_ver%%.*}"
if [[ -z "$xcode_major" || "$xcode_major" -lt 15 ]]; then
	echo "xcode: need Xcode 15+, found '${xcode_ver:-none}' at $MAYBRAID_XCODE" >&2
	exit 1
fi

echo "Xcode $xcode_ver ($DEVELOPER_DIR)"
echo "SDKROOT=$SDKROOT deployment=$MACOSX_DEPLOYMENT_TARGET"

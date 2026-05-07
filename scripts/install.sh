#!/usr/bin/env bash
#
# Quick-try installer for wjmclock — downloads the right pre-built binary
# (x86_64 or aarch64), grabs the example config, checks runtime libraries,
# and launches the app.
#
# Designed for `curl … | bash`. Does NOT require sudo and never installs
# anything system-wide; if a runtime library is missing it prints the
# suggested `apt install` command and exits without touching the system.
#
# Override the install location with WJMCLOCK_DIR (default: ~/wjmclock).
#
#   curl -fsSL https://raw.githubusercontent.com/ast/wjmclock/main/scripts/install.sh | bash

set -euo pipefail

readonly REPO="ast/wjmclock"
readonly PREFIX="${WJMCLOCK_DIR:-$HOME/wjmclock}"

# ─── arch detection ──────────────────────────────────────────────────
ARCH=$(uname -m)
case "$ARCH" in
  x86_64)  TARGET=x86_64-unknown-linux-gnu ;;
  aarch64) TARGET=aarch64-unknown-linux-gnu ;;
  *)
    echo "wjmclock: unsupported arch '$ARCH' (need x86_64 or aarch64)" >&2
    exit 1
    ;;
esac

# ─── download ────────────────────────────────────────────────────────
mkdir -p "$PREFIX"
cd "$PREFIX"

binary_url="https://github.com/${REPO}/releases/latest/download/wjmclock-${TARGET}-stripped"
config_url="https://raw.githubusercontent.com/${REPO}/main/wjmclock.example.toml"

echo "→ downloading wjmclock (${TARGET}, stripped) into $PREFIX"
if ! curl --fail --silent --show-error --location -o wjmclock "$binary_url"; then
  echo
  echo "  ✗ failed to download $binary_url"
  echo "    Check that a release exists at https://github.com/${REPO}/releases/latest"
  exit 1
fi
if ! curl --fail --silent --show-error --location -o wjmclock.toml "$config_url"; then
  echo
  echo "  ✗ failed to download $config_url"
  exit 1
fi
chmod +x wjmclock

# ─── runtime-library check (no sudo, no install) ─────────────────────
# egui/glow/winit `dlopen` GL, EGL, Wayland, X11 etc. at runtime, so they
# don't appear in `ldd` output. Probe each one by querying the dynamic
# linker cache (`ldconfig -p`) and falling back to the standard lib
# directories.
#
# Map .so soname → Debian / Ubuntu / Pi OS package, picked from
# Cross.toml + the README runtime list.
declare -A SO_TO_PKG=(
  [libGL.so.1]=libgl1
  [libEGL.so.1]=libegl1
  [libwayland-client.so.0]=libwayland-client0
  [libxkbcommon.so.0]=libxkbcommon0
  [libxkbcommon-x11.so.0]=libxkbcommon-x11-0
  [libXcursor.so.1]=libxcursor1
  [libXi.so.6]=libxi6
  [libXrandr.so.2]=libxrandr2
  [libxcb-render.so.0]=libxcb-render0
  [libxcb-shape.so.0]=libxcb-shape0
  [libxcb-xfixes.so.0]=libxcb-xfixes0
)
needed_libs=(
  libGL.so.1 libEGL.so.1
  libwayland-client.so.0
  libxkbcommon.so.0 libxkbcommon-x11.so.0
  libXcursor.so.1 libXi.so.6 libXrandr.so.2
  libxcb-render.so.0 libxcb-shape.so.0 libxcb-xfixes.so.0
)

# Cache `ldconfig -p` once (it's a slow-ish syscall heavy command).
ldconfig_cache=""
if command -v ldconfig >/dev/null 2>&1; then
  ldconfig_cache=$(ldconfig -p 2>/dev/null || true)
fi

# Standard lib search paths, including multi-arch dirs Debian/Ubuntu use.
arch_dir=$(uname -m)-linux-gnu
fs_search_paths=(
  /lib /usr/lib
  "/lib/$arch_dir" "/usr/lib/$arch_dir"
  /lib64 /usr/lib64
)

have_lib() {
  local lib="$1"
  if [ -n "$ldconfig_cache" ] && grep -q -F "$lib " <<< "$ldconfig_cache"; then
    return 0
  fi
  local d
  for d in "${fs_search_paths[@]}"; do
    [ -e "$d/$lib" ] && return 0
  done
  return 1
}

echo "→ checking runtime libraries"
missing_libs=()
for lib in "${needed_libs[@]}"; do
  if ! have_lib "$lib"; then
    missing_libs+=("$lib")
  fi
done

if [ "${#missing_libs[@]}" -gt 0 ]; then
  echo
  echo "  ✗ wjmclock can't start yet — these libraries are missing:"
  printf '      %s\n' "${missing_libs[@]}"

  pkgs=()
  unmapped=()
  for lib in "${missing_libs[@]}"; do
    pkg="${SO_TO_PKG[$lib]:-}"
    if [ -n "$pkg" ]; then
      pkgs+=("$pkg")
    else
      unmapped+=("$lib")
    fi
  done

  if [ "${#pkgs[@]}" -gt 0 ]; then
    # de-duplicate while preserving order
    uniq_pkgs=$(printf '%s\n' "${pkgs[@]}" | awk '!seen[$0]++' | tr '\n' ' ')
    echo
    echo "  On Debian / Ubuntu / Pi OS, run:"
    echo "      sudo apt install ${uniq_pkgs% }"
  fi
  if [ "${#unmapped[@]}" -gt 0 ]; then
    echo
    echo "  Other distros: install the system packages providing these"
    echo "  libraries:"
    printf '      %s\n' "${unmapped[@]}"
  fi
  echo
  echo "  Once installed, launch with:"
  echo "      cd $PREFIX && ./wjmclock"
  exit 1
fi

echo "→ all dependencies present — launching"
exec ./wjmclock

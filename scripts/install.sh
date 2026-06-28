#!/usr/bin/env bash
set -euo pipefail

REPO="brilyyy/omah"
BIN="omah"
INSTALL_DIR="${INSTALL_DIR:-~/.local/bin}"

# -- Parse flags ---------------------------------------------------------
BUILD_SOURCE=false
while [ $# -gt 0 ]; do
  case "$1" in
    --source) BUILD_SOURCE=true; shift ;;
    --prefix=*) INSTALL_DIR="${1#*=}"; shift ;;
    --prefix) INSTALL_DIR="$2"; shift 2 ;;
    *) echo "Usage: $0 [--source] [--prefix <dir>]"; exit 1 ;;
  esac
done

if [ "$BUILD_SOURCE" = true ]; then
  # -- Build from source --------------------------------------------------
  echo "> Cloning ${REPO}..."
  TMP="$(mktemp -d)"
  git clone --depth 1 "https://github.com/${REPO}.git" "${TMP}"

  echo "> Building..."
  (cd "${TMP}" && cargo build --release --bin omah)

  echo "> Installing ${BIN} to ${INSTALL_DIR}..."
  install -d "${INSTALL_DIR}"
  install -m 755 "${TMP}/target/release/${BIN}" "${INSTALL_DIR}/${BIN}"

  rm -rf "${TMP}"
  echo "* ${BIN} installed at ${INSTALL_DIR}/${BIN} (built from source)"
  exit 0
fi

# -- Platform detection ---------------------------------------------------
ARCH=""
case "$(uname -s)-$(uname -m)" in
  Darwin-arm64)  ARCH="macos-aarch64"  ;;
  Darwin-x86_64) ARCH="macos-x86_64"   ;;
  Linux-x86_64)  ARCH="linux-x86_64"   ;;
  *)
    echo "Unsupported platform: $(uname -s)-$(uname -m)"
    exit 1
    ;;
esac

# -- Fetch latest release tag ---------------------------------------------
echo "> Detecting latest release..."
TAG="$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
  | grep '"tag_name":' | sed 's/.*"tag_name": "//;s/".*//')"

if [ -z "${TAG}" ]; then
  echo "x Failed to detect latest release tag"
  exit 1
fi
echo "  Latest: ${TAG}"

# -- Download & extract ---------------------------------------------------
TARBALL="${BIN}-${TAG#v}-${ARCH}.tar.gz"
URL="https://github.com/${REPO}/releases/download/${TAG}/${TARBALL}"
TMP="$(mktemp -d)"

echo "> Downloading ${TARBALL}..."
curl -fsSL "${URL}" -o "${TMP}/${TARBALL}"

echo "> Extracting..."
tar -xzf "${TMP}/${TARBALL}" -C "${TMP}"

# -- Install --------------------------------------------------------------
echo "> Installing ${BIN} to ${INSTALL_DIR}..."
install -d "${INSTALL_DIR}"
install -m 755 "${TMP}/${BIN}" "${INSTALL_DIR}/${BIN}"

# -- Cleanup --------------------------------------------------------------
rm -rf "${TMP}"

echo "* ${BIN} ${TAG} installed at ${INSTALL_DIR}/${BIN}"

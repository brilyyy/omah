#!/usr/bin/env bash
# omah installer — https://github.com/brilyyy/omah
set -euo pipefail

REPO="brilyyy/omah"
BIN_NAME="omah"
INSTALL_DIR="${HOME}/.local/bin"

RED='\033[0;31m'
GRN='\033[0;32m'
YLW='\033[1;33m'
BLU='\033[0;34m'
CYN='\033[0;36m'
BLD='\033[1m'
DIM='\033[2m'
NC='\033[0m'

info()    { echo -e "${BLU}→${NC} $*"; }
success() { echo -e "${GRN}✓${NC} $*"; }
warn()    { echo -e "${YLW}!${NC} $*"; }
error()   { echo -e "${RED}✗${NC} $*" >&2; }
die()     { error "$*"; exit 1; }
dim()     { echo -e "${DIM}$*${NC}"; }

banner() {
  echo
  echo -e "${BLD}${CYN}          _     ${NC}"
  echo -e "${BLD}${CYN}  ___  _ | |_   ${NC}  omah — dotfile manager"
  echo -e "${BLD}${CYN} / _ \| || ' \\  ${NC}  github.com/${REPO}"
  echo -e "${BLD}${CYN} \\___/|_||_||_| ${NC}"
  echo
}

detect_platform() {
  OS="$(uname -s)"
  ARCH="$(uname -m)"

  case "${OS}" in
    Linux)  OS="linux"  ;;
    Darwin) OS="macos"  ;;
    *)      die "Unsupported OS: ${OS}" ;;
  esac

  case "${ARCH}" in
    x86_64 | amd64)  ARCH="x86_64"  ;;
    aarch64 | arm64) ARCH="aarch64" ;;
    *)               die "Unsupported architecture: ${ARCH}" ;;
  esac

  if [[ "${OS}" == "linux" && "${ARCH}" == "x86_64" ]]; then
    TARGET="x86_64-unknown-linux-musl"
  elif [[ "${OS}" == "macos" && "${ARCH}" == "aarch64" ]]; then
    TARGET="aarch64-apple-darwin"
  elif [[ "${OS}" == "macos" && "${ARCH}" == "x86_64" ]]; then
    TARGET="x86_64-apple-darwin"
  else
    die "No prebuilt binary for ${OS}-${ARCH}. Try building from source."
  fi
}

need_cmd() {
  if ! command -v "$1" &>/dev/null; then
    return 1
  fi
  return 0
}

require_cmd() {
  need_cmd "$1" || die "Required command not found: $1. Please install it and retry."
}

fetch() {
  if need_cmd curl; then
    curl --proto '=https' --tlsv1.2 -fsSL "$@"
  elif need_cmd wget; then
    wget -qO- "$@"
  else
    die "Neither curl nor wget found. Please install one and retry."
  fi
}

ensure_install_dir() {
  local dir="$1"
  if [[ ! -d "${dir}" ]]; then
    info "Creating ${dir}"
    mkdir -p "${dir}"
  fi
}

add_to_path_hint() {
  local dir="$1"
  if [[ ":${PATH}:" != *":${dir}:"* ]]; then
    echo
    warn "${dir} is not in your PATH."
    echo -e "  Add this to your shell config (${DIM}~/.zshrc${NC} / ${DIM}~/.bashrc${NC}):"
    echo -e "  ${CYN}export PATH=\"${dir}:\$PATH\"${NC}"
    echo
  fi
}

get_latest_version() {
  local api_url="https://api.github.com/repos/${REPO}/releases/latest"
  local version
  version=$(fetch "${api_url}" | grep '"tag_name"' | sed 's/.*"tag_name": *"\(.*\)".*/\1/')
  if [[ -z "${version}" ]]; then
    die "Could not determine latest release version. Check your internet connection."
  fi
  echo "${version}"
}

install_cli_prebuilt() {
  info "Fetching latest release version…"
  local version
  version=$(get_latest_version)
  success "Latest version: ${version}"

  local archive="omah-${version}-${TARGET}.tar.gz"
  local url="https://github.com/${REPO}/releases/download/${version}/${archive}"
  local tmp
  tmp="$(mktemp -d)"
  trap 'rm -rf "${tmp}"' EXIT

  info "Downloading ${archive}…"
  fetch "${url}" -o "${tmp}/${archive}" || die "Download failed."

  info "Extracting…"
  tar -xzf "${tmp}/${archive}" -C "${tmp}"

  ensure_install_dir "${INSTALL_DIR}"
  install -m 755 "${tmp}/${BIN_NAME}" "${INSTALL_DIR}/${BIN_NAME}"

  success "Installed omah ${version} → ${INSTALL_DIR}/${BIN_NAME}"
  add_to_path_hint "${INSTALL_DIR}"
}

install_cli_source() {
  require_cmd cargo

  local src_dir
  src_dir="$(mktemp -d)"
  trap 'rm -rf "${src_dir}"' EXIT

  info "Cloning repository…"
  if need_cmd git; then
    git clone --depth 1 "https://github.com/${REPO}.git" "${src_dir}" -q
  else
    die "git is required to build from source."
  fi

  info "Building omah…"
  (
    cd "${src_dir}"
    cargo build --bin omah --release --locked 2>&1 | grep -E "^(Compiling|Finished|error)" || true
  )

  ensure_install_dir "${INSTALL_DIR}"
  install -m 755 "${src_dir}/target/release/${BIN_NAME}" "${INSTALL_DIR}/${BIN_NAME}"

  success "Built and installed omah → ${INSTALL_DIR}/${BIN_NAME}"
  add_to_path_hint "${INSTALL_DIR}"
}

main() {
  banner
  detect_platform
  dim "Detected: ${OS} / ${ARCH}"
  echo

  echo -e "${BLD}Install method:${NC}"
  echo
  echo -e "  ${BLD}1)${NC} Download prebuilt binary  ${DIM}(fast, ~2 MB)${NC}"
  echo -e "  ${BLD}2)${NC} Build from source          ${DIM}(requires Rust)${NC}"
  echo
  read -rp "$(echo -e "Your choice ${DIM}[1/2, default: 1]${NC}: ")" choice
  choice="${choice:-1}"

  echo
  case "${choice}" in
    1) install_cli_prebuilt ;;
    2) install_cli_source   ;;
    *) warn "Invalid choice, defaulting to prebuilt."; install_cli_prebuilt ;;
  esac

  echo
  success "Done! Run ${CYN}omah --help${NC} to get started."
  dim "  omah init       — set up config"
  dim "  omah backup     — back up dotfiles"
  dim "  omah restore    — restore dotfiles"
  dim "  omah status     — check sync status"
  echo
}

main "$@"

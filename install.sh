#!/usr/bin/env bash
# omah installer — https://github.com/brilyyy/omah
set -euo pipefail

# ── Config ────────────────────────────────────────────────────────────────────

REPO="brilyyy/omah"
BIN_NAME="omah"
INSTALL_DIR="${HOME}/.local/bin"
MACOS_APP_DIR="/Applications"

# ── Colors ────────────────────────────────────────────────────────────────────

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

# ── Banner ────────────────────────────────────────────────────────────────────

banner() {
  echo
  echo -e "${BLD}${CYN}          _     ${NC}"
  echo -e "${BLD}${CYN}  ___  _ | |_   ${NC}  omah — dotfile manager"
  echo -e "${BLD}${CYN} / _ \| || ' \\  ${NC}  github.com/${REPO}"
  echo -e "${BLD}${CYN} \\___/|_||_||_| ${NC}"
  echo
}

# ── OS / Arch detection ───────────────────────────────────────────────────────

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

# ── Helpers ───────────────────────────────────────────────────────────────────

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
  # Prefer curl, fall back to wget
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

# ── Latest version ────────────────────────────────────────────────────────────

get_latest_version() {
  local api_url="https://api.github.com/repos/${REPO}/releases/latest"
  local version
  version=$(fetch "${api_url}" | grep '"tag_name"' | sed 's/.*"tag_name": *"\(.*\)".*/\1/')
  if [[ -z "${version}" ]]; then
    die "Could not determine latest release version. Check your internet connection."
  fi
  echo "${version}"
}

# ── CLI install (prebuilt binary) ─────────────────────────────────────────────

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
  fetch "${url}" -o "${tmp}/${archive}" || die "Download failed. The release may not include a binary for ${TARGET}."

  info "Extracting…"
  tar -xzf "${tmp}/${archive}" -C "${tmp}"

  ensure_install_dir "${INSTALL_DIR}"
  install -m 755 "${tmp}/${BIN_NAME}" "${INSTALL_DIR}/${BIN_NAME}"

  success "Installed omah ${version} → ${INSTALL_DIR}/${BIN_NAME}"
  add_to_path_hint "${INSTALL_DIR}"
}

# ── CLI install (build from source, no TUI) ───────────────────────────────────

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

  info "Building omah (CLI, no TUI)…"
  (
    cd "${src_dir}"
    cargo build --bin omah --release --locked 2>&1 | grep -E "^(Compiling|Finished|error)" || true
  )

  ensure_install_dir "${INSTALL_DIR}"
  install -m 755 "${src_dir}/target/release/${BIN_NAME}" "${INSTALL_DIR}/${BIN_NAME}"

  success "Built and installed omah → ${INSTALL_DIR}/${BIN_NAME}"
  add_to_path_hint "${INSTALL_DIR}"
}

# ── GUI install (build Tauri app from source) ─────────────────────────────────

install_gui_source() {
  echo
  info "Building the omah Desktop app from source requires:"
  echo -e "  ${DIM}• Rust (cargo)${NC}"
  echo -e "  ${DIM}• Bun (or Node.js + npm)${NC}"
  if [[ "${OS}" == "linux" ]]; then
    echo -e "  ${DIM}• webkit2gtk-4.1, libssl-dev, build-essential (and other Tauri deps)${NC}"
    echo -e "  ${DIM}  See: https://v2.tauri.app/start/prerequisites/#linux${NC}"
  elif [[ "${OS}" == "macos" ]]; then
    echo -e "  ${DIM}• Xcode Command Line Tools${NC}"
  fi
  echo

  # Check required tools
  local missing=()
  need_cmd cargo  || missing+=("cargo (https://rustup.rs)")
  need_cmd git    || missing+=("git")
  if need_cmd bun; then
    PKG_CMD="bun"
    PKG_INSTALL="bun install"
    PKG_BUILD="bun run"
  elif need_cmd npm; then
    PKG_CMD="npm"
    PKG_INSTALL="npm install"
    PKG_BUILD="npm run"
  else
    missing+=("bun (https://bun.sh) or Node.js")
  fi

  if [[ ${#missing[@]} -gt 0 ]]; then
    error "Missing prerequisites:"
    for dep in "${missing[@]}"; do
      echo -e "  ${RED}•${NC} ${dep}"
    done
    exit 1
  fi

  read -rp "$(echo -e "\n${YLW}?${NC} Continue with build? [y/N] ")" confirm
  [[ "${confirm}" =~ ^[Yy]$ ]] || { info "Aborted."; exit 0; }

  local src_dir
  src_dir="$(mktemp -d)"
  trap 'rm -rf "${src_dir}"' EXIT

  info "Cloning repository…"
  git clone --depth 1 "https://github.com/${REPO}.git" "${src_dir}" -q

  info "Installing frontend dependencies…"
  (cd "${src_dir}/apps/desktop" && ${PKG_INSTALL} --silent)

  info "Building Tauri application (this may take a few minutes)…"
  (cd "${src_dir}/apps/desktop" && ${PKG_BUILD} tauri build 2>&1 | grep -E "^(Compiling|Bundling|Finished|error)" || true)

  if [[ "${OS}" == "macos" ]]; then
    local app_path
    app_path="$(find "${src_dir}/apps/desktop/src-tauri/target/release/bundle/macos" -name "*.app" -maxdepth 1 2>/dev/null | head -1)"
    if [[ -n "${app_path}" ]]; then
      info "Installing to ${MACOS_APP_DIR}/omah.app…"
      cp -r "${app_path}" "${MACOS_APP_DIR}/omah.app"
      success "Installed omah.app → ${MACOS_APP_DIR}"
    else
      die "Could not find built .app bundle."
    fi

  elif [[ "${OS}" == "linux" ]]; then
    # Prefer AppImage, then deb
    local appimage
    appimage="$(find "${src_dir}/apps/desktop/src-tauri/target/release/bundle/appimage" -name "*.AppImage" 2>/dev/null | head -1)"
    if [[ -n "${appimage}" ]]; then
      ensure_install_dir "${INSTALL_DIR}"
      install -m 755 "${appimage}" "${INSTALL_DIR}/omah.AppImage"
      # Optional: create a wrapper script so `omah-gui` launches it
      cat > "${INSTALL_DIR}/omah-gui" <<'EOF'
#!/usr/bin/env sh
exec "$(dirname "$0")/omah.AppImage" "$@"
EOF
      chmod +x "${INSTALL_DIR}/omah-gui"
      success "Installed omah AppImage → ${INSTALL_DIR}/omah.AppImage"
      add_to_path_hint "${INSTALL_DIR}"
    else
      # Try deb
      local deb
      deb="$(find "${src_dir}/apps/desktop/src-tauri/target/release/bundle/deb" -name "*.deb" 2>/dev/null | head -1)"
      if [[ -n "${deb}" && "$(id -u)" -eq 0 ]]; then
        dpkg -i "${deb}"
        success "Installed .deb package."
      elif [[ -n "${deb}" ]]; then
        info "Run the following to install the .deb package:"
        echo -e "  ${CYN}sudo dpkg -i ${deb}${NC}"
        cp "${deb}" "${HOME}/omah.deb"
        success "Copied omah.deb → ${HOME}/omah.deb"
      else
        die "Could not find a built bundle. Check ${src_dir}/apps/desktop/src-tauri/target/release/bundle/"
      fi
    fi
  fi
}

# ── Interactive prompt ────────────────────────────────────────────────────────

choose_install_type() {
  echo -e "${BLD}How would you like to install omah?${NC}"
  echo
  echo -e "  ${BLD}1)${NC} Desktop GUI  ${DIM}— Tauri app with full visual interface${NC}"
  echo -e "  ${BLD}2)${NC} CLI only     ${DIM}— lightweight terminal binary, no TUI${NC}"
  echo
  read -rp "$(echo -e "Your choice ${DIM}[1/2, default: 2]${NC}: ")" choice
  choice="${choice:-2}"

  case "${choice}" in
    1) INSTALL_TYPE="gui"  ;;
    2) INSTALL_TYPE="cli"  ;;
    *) warn "Invalid choice, defaulting to CLI."; INSTALL_TYPE="cli" ;;
  esac
}

choose_cli_source() {
  echo
  echo -e "${BLD}CLI install method:${NC}"
  echo
  echo -e "  ${BLD}1)${NC} Download prebuilt binary  ${DIM}(fast, ~2 MB)${NC}"
  echo -e "  ${BLD}2)${NC} Build from source          ${DIM}(requires Rust, no TUI feature)${NC}"
  echo
  read -rp "$(echo -e "Your choice ${DIM}[1/2, default: 1]${NC}: ")" src_choice
  src_choice="${src_choice:-1}"

  case "${src_choice}" in
    1) CLI_SOURCE="prebuilt" ;;
    2) CLI_SOURCE="source"   ;;
    *) warn "Invalid choice, defaulting to prebuilt."; CLI_SOURCE="prebuilt" ;;
  esac
}

# ── Main ──────────────────────────────────────────────────────────────────────

main() {
  banner
  detect_platform
  dim "Detected: ${OS} / ${ARCH}"
  echo

  choose_install_type

  case "${INSTALL_TYPE}" in
    cli)
      choose_cli_source
      echo
      case "${CLI_SOURCE}" in
        prebuilt) install_cli_prebuilt ;;
        source)   install_cli_source   ;;
      esac
      echo
      success "Done! Run ${CYN}omah --help${NC} to get started."
      dim "  omah init       — set up config"
      dim "  omah backup     — back up dotfiles"
      dim "  omah restore    — restore dotfiles"
      dim "  omah status     — check sync status"
      ;;

    gui)
      echo
      install_gui_source
      echo
      success "Done! Launch omah from your Applications."
      ;;
  esac

  echo
}

main "$@"

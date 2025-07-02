#!/bin/sh
# Script Language Installer for Unix-like systems (Linux, macOS, BSD)

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m' # No Color

# Configuration
REPO_OWNER="moikapy"
REPO_NAME="script"
GITHUB_API="https://api.github.com/repos/${REPO_OWNER}/${REPO_NAME}"
INSTALL_DIR="${SCRIPT_INSTALL_DIR:-$HOME/.local/bin}"
TEMP_DIR=$(mktemp -d)

# Cleanup on exit
trap 'rm -rf "$TEMP_DIR"' EXIT

# Helper functions
info() {
    printf "${BLUE}${BOLD}==>${NC} ${BOLD}%s${NC}\n" "$1"
}

success() {
    printf "${GREEN}${BOLD}✓${NC} %s\n" "$1"
}

error() {
    printf "${RED}${BOLD}✗${NC} %s\n" "$1" >&2
}

warning() {
    printf "${YELLOW}${BOLD}!${NC} %s\n" "$1"
}

# Detect OS and architecture
detect_platform() {
    OS=$(uname -s | tr '[:upper:]' '[:lower:]')
    ARCH=$(uname -m)
    
    case "$OS" in
        linux*)
            PLATFORM="linux"
            ;;
        darwin*)
            PLATFORM="macos"
            ;;
        freebsd*)
            PLATFORM="freebsd"
            ;;
        *)
            error "Unsupported operating system: $OS"
            exit 1
            ;;
    esac
    
    case "$ARCH" in
        x86_64|amd64)
            ARCH="amd64"
            ;;
        aarch64|arm64)
            ARCH="arm64"
            ;;
        *)
            error "Unsupported architecture: $ARCH"
            exit 1
            ;;
    esac
    
    # Check if we should use musl for Linux
    if [ "$PLATFORM" = "linux" ] && ldd --version 2>&1 | grep -q musl; then
        PLATFORM_SUFFIX="${PLATFORM}-${ARCH}-musl"
    else
        PLATFORM_SUFFIX="${PLATFORM}-${ARCH}"
    fi
    
    info "Detected platform: $PLATFORM_SUFFIX"
}

# Check for required tools
check_requirements() {
    info "Checking requirements..."
    
    if ! command -v curl >/dev/null 2>&1 && ! command -v wget >/dev/null 2>&1; then
        error "Neither curl nor wget found. Please install one of them."
        exit 1
    fi
    
    if ! command -v tar >/dev/null 2>&1; then
        error "tar is required but not found. Please install it."
        exit 1
    fi
    
    success "All requirements met"
}

# Download file with curl or wget
download() {
    local url="$1"
    local output="$2"
    
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL "$url" -o "$output"
    else
        wget -q "$url" -O "$output"
    fi
}

# Get the latest release version
get_latest_version() {
    info "Fetching latest version..."
    
    local api_url="${GITHUB_API}/releases/latest"
    local version
    
    if command -v curl >/dev/null 2>&1; then
        version=$(curl -fsSL "$api_url" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')
    else
        version=$(wget -qO- "$api_url" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')
    fi
    
    if [ -z "$version" ]; then
        error "Failed to fetch latest version"
        exit 1
    fi
    
    echo "$version"
}

# Download and install Script
install_script() {
    local version="${1:-$(get_latest_version)}"
    
    success "Installing Script Language $version"
    
    # Construct download URL
    local archive_name="script-${PLATFORM_SUFFIX}.tar.gz"
    local download_url="https://github.com/${REPO_OWNER}/${REPO_NAME}/releases/download/${version}/${archive_name}"
    
    info "Downloading from: $download_url"
    
    # Download archive
    cd "$TEMP_DIR"
    if ! download "$download_url" "$archive_name"; then
        error "Failed to download Script Language"
        exit 1
    fi
    
    # Download and verify checksum
    if download "${download_url}.sha256" "${archive_name}.sha256" 2>/dev/null; then
        info "Verifying checksum..."
        
        if command -v sha256sum >/dev/null 2>&1; then
            sha256sum -c "${archive_name}.sha256"
        elif command -v shasum >/dev/null 2>&1; then
            shasum -a 256 -c "${archive_name}.sha256"
        else
            warning "Cannot verify checksum: sha256sum/shasum not found"
        fi
    else
        warning "Checksum file not found, skipping verification"
    fi
    
    # Extract archive
    info "Extracting archive..."
    tar -xzf "$archive_name"
    
    # Create install directory if needed
    if [ ! -d "$INSTALL_DIR" ]; then
        info "Creating install directory: $INSTALL_DIR"
        mkdir -p "$INSTALL_DIR"
    fi
    
    # Install binaries
    info "Installing binaries to $INSTALL_DIR..."
    
    for binary in script script-lsp manuscript; do
        if [ -f "$binary" ]; then
            cp "$binary" "$INSTALL_DIR/"
            chmod +x "$INSTALL_DIR/$binary"
            success "Installed $binary"
        else
            warning "$binary not found in archive"
        fi
    done
}

# Update PATH if needed
update_path() {
    # Check if install dir is in PATH
    if echo "$PATH" | grep -q "$INSTALL_DIR"; then
        return
    fi
    
    info "Adding $INSTALL_DIR to PATH..."
    
    # Detect shell and update appropriate config file
    local shell_config=""
    case "$SHELL" in
        */bash)
            shell_config="$HOME/.bashrc"
            ;;
        */zsh)
            shell_config="$HOME/.zshrc"
            ;;
        */fish)
            shell_config="$HOME/.config/fish/config.fish"
            ;;
        *)
            shell_config="$HOME/.profile"
            ;;
    esac
    
    if [ -f "$shell_config" ]; then
        echo "" >> "$shell_config"
        echo "# Added by Script Language installer" >> "$shell_config"
        echo "export PATH=\"\$PATH:$INSTALL_DIR\"" >> "$shell_config"
        
        warning "PATH updated in $shell_config"
        warning "Run 'source $shell_config' or restart your shell to use Script"
    else
        warning "Could not update PATH automatically"
        warning "Add the following to your shell configuration:"
        echo "    export PATH=\"\$PATH:$INSTALL_DIR\""
    fi
}

# Main installation process
main() {
    echo ""
    echo "${CYAN}${BOLD}Script Language Installer${NC}"
    echo ""
    
    check_requirements
    detect_platform
    
    # Parse command line arguments
    VERSION=""
    while [ $# -gt 0 ]; do
        case "$1" in
            --version|-v)
                VERSION="$2"
                shift 2
                ;;
            --dir|-d)
                INSTALL_DIR="$2"
                shift 2
                ;;
            --help|-h)
                echo "Usage: $0 [options]"
                echo ""
                echo "Options:"
                echo "  --version, -v <version>    Install specific version"
                echo "  --dir, -d <directory>      Install directory (default: $INSTALL_DIR)"
                echo "  --help, -h                 Show this help message"
                exit 0
                ;;
            *)
                error "Unknown option: $1"
                exit 1
                ;;
        esac
    done
    
    install_script "$VERSION"
    update_path
    
    echo ""
    success "${GREEN}${BOLD}Script Language installed successfully!${NC}"
    echo ""
    echo "Run '${BOLD}script --version${NC}' to verify the installation"
    echo "Run '${BOLD}script update${NC}' to check for updates"
    echo ""
}

# Run main installation
main "$@"
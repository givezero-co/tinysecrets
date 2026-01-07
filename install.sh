#!/bin/sh
# TinySecrets Installer
# Usage: curl -sSfL https://raw.githubusercontent.com/givezero-co/tinysecrets/main/install.sh | sh
#
# This script downloads and installs the tinysecrets (ts) binary.
# It supports macOS (arm64, x86_64) and Linux (x86_64, arm64).

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color
BOLD='\033[1m'

# Configuration
REPO="givezero-co/tinysecrets"
BINARY_NAME="tinysecrets"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"

# Detect OS and architecture
detect_platform() {
    OS="$(uname -s)"
    ARCH="$(uname -m)"

    case "$OS" in
        Darwin)
            OS="apple-darwin"
            ;;
        Linux)
            OS="unknown-linux-gnu"
            ;;
        *)
            printf "${RED}Error: Unsupported operating system: %s${NC}\n" "$OS" >&2
            exit 1
            ;;
    esac

    case "$ARCH" in
        x86_64|amd64)
            ARCH="x86_64"
            ;;
        arm64|aarch64)
            ARCH="aarch64"
            ;;
        *)
            printf "${RED}Error: Unsupported architecture: %s${NC}\n" "$ARCH" >&2
            exit 1
            ;;
    esac

    PLATFORM="${ARCH}-${OS}"
    printf "${CYAN}Detected platform:${NC} %s\n" "$PLATFORM"
}

# Get the latest release version from GitHub
get_latest_version() {
    VERSION=$(curl -sSf "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | sed -E 's/.*"v([^"]+)".*/\1/')
    
    if [ -z "$VERSION" ]; then
        printf "${RED}Error: Could not determine latest version${NC}\n" >&2
        exit 1
    fi
    
    printf "${CYAN}Latest version:${NC} v%s\n" "$VERSION"
}

# Download and install the binary
install_binary() {
    DOWNLOAD_URL="https://github.com/${REPO}/releases/download/v${VERSION}/${BINARY_NAME}-${VERSION}-${PLATFORM}.tar.gz"
    
    printf "${CYAN}Downloading:${NC} %s\n" "$DOWNLOAD_URL"
    
    # Create temp directory
    TMP_DIR=$(mktemp -d)
    trap "rm -rf $TMP_DIR" EXIT
    
    # Download and extract
    if ! curl -sSfL "$DOWNLOAD_URL" | tar xz -C "$TMP_DIR" 2>/dev/null; then
        printf "${RED}Error: Failed to download release${NC}\n" >&2
        printf "This could mean:\n" >&2
        printf "  - No pre-built binary for your platform yet\n" >&2
        printf "  - Network issues\n" >&2
        printf "\nYou can build from source instead:\n" >&2
        printf "  ${CYAN}cargo install --git https://github.com/${REPO}${NC}\n" >&2
        exit 1
    fi
    
    # Create install directory
    mkdir -p "$INSTALL_DIR"
    
    # Install binary
    mv "$TMP_DIR/$BINARY_NAME" "$INSTALL_DIR/$BINARY_NAME"
    chmod +x "$INSTALL_DIR/$BINARY_NAME"
    
    printf "${GREEN}${BOLD}✓${NC} Installed ${BOLD}%s${NC} to ${CYAN}%s${NC}\n" "$BINARY_NAME" "$INSTALL_DIR/$BINARY_NAME"
}

# Check if install dir is in PATH
check_path() {
    case ":$PATH:" in
        *":$INSTALL_DIR:"*)
            # Already in PATH
            ;;
        *)
            printf "\n${YELLOW}Note:${NC} %s is not in your PATH\n" "$INSTALL_DIR"
            printf "Add it to your shell config:\n\n"
            
            SHELL_NAME=$(basename "$SHELL")
            case "$SHELL_NAME" in
                zsh)
                    printf "  ${CYAN}echo 'export PATH=\"\$HOME/.local/bin:\$PATH\"' >> ~/.zshrc${NC}\n"
                    printf "  ${CYAN}source ~/.zshrc${NC}\n"
                    ;;
                bash)
                    printf "  ${CYAN}echo 'export PATH=\"\$HOME/.local/bin:\$PATH\"' >> ~/.bashrc${NC}\n"
                    printf "  ${CYAN}source ~/.bashrc${NC}\n"
                    ;;
                fish)
                    printf "  ${CYAN}fish_add_path ~/.local/bin${NC}\n"
                    ;;
                *)
                    printf "  ${CYAN}export PATH=\"\$HOME/.local/bin:\$PATH\"${NC}\n"
                    ;;
            esac
            printf "\n"
            ;;
    esac
}

# Print success message
print_success() {
    printf "\n"
    printf "${GREEN}${BOLD}TinySecrets installed successfully!${NC}\n"
    printf "\n"
    printf "${BOLD}Quick start:${NC}\n"
    printf "  ${CYAN}ts init${NC}                           # Create encrypted store\n"
    printf "  ${CYAN}ts set myapp staging API_KEY${NC}      # Set a secret\n"
    printf "  ${CYAN}ts run -p myapp -e staging -- cmd${NC} # Run with secrets\n"
    printf "\n"
    printf "For more info: ${CYAN}ts --help${NC}\n"
}

# Main
main() {
    printf "\n${BOLD}Installing TinySecrets...${NC}\n\n"
    
    detect_platform
    get_latest_version
    install_binary
    check_path
    print_success
}

main


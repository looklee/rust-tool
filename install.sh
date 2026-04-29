#!/bin/bash

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

print_banner() {
    echo -e "${BLUE}"
    echo "╔═══════════════════════════════════════════════════════════╗"
    echo "║           AI Coding Assistant - Installer                ║"
    echo "║                     Version 0.1.0                        ║"
    echo "╚═══════════════════════════════════════════════════════════╝"
    echo -e "${NC}"
}

check_root() {
    if [ "$(id -u)" != "0" ]; then
        echo -e "${YELLOW}Warning: Installing to system path requires root privileges.${NC}"
        echo -e "${YELLOW}You may need to run with sudo.${NC}"
        echo ""
    fi
}

check_rust() {
    echo -e "${CYAN}Checking Rust installation...${NC}"
    if ! command -v rustc &> /dev/null; then
        echo -e "${YELLOW}Rust not found. Installing Rust...${NC}"
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source "$HOME/.cargo/env"
        echo -e "${GREEN}Rust installed successfully!${NC}"
    else
        echo -e "${GREEN}Rust is already installed.${NC}"
    fi
}

build_project() {
    echo -e "${CYAN}Building project...${NC}"
    cargo build --release -p common
    cargo build --release -p code
    
    if [ -f "target/release/code" ]; then
        echo -e "${GREEN}Build successful!${NC}"
    else
        echo -e "${RED}Build failed!${NC}"
        exit 1
    fi
}

install_binary() {
    echo -e "${CYAN}Installing binary...${NC}"
    
    if [ -d "/usr/local/bin" ]; then
        INSTALL_PATH="/usr/local/bin/code-ai"
    elif [ -d "$HOME/.local/bin" ]; then
        INSTALL_PATH="$HOME/.local/bin/code-ai"
    else
        INSTALL_PATH="$HOME/bin/code-ai"
        mkdir -p "$HOME/bin"
    fi
    
    echo -e "${YELLOW}Installing to: $INSTALL_PATH${NC}"
    
    if [ "$(id -u)" = "0" ]; then
        cp "target/release/code" "$INSTALL_PATH"
        chmod 755 "$INSTALL_PATH"
    else
        cp "target/release/code" "$INSTALL_PATH"
        chmod 755 "$INSTALL_PATH"
    fi
    
    echo -e "${GREEN}Binary installed successfully!${NC}"
    
    if echo "$PATH" | grep -q "$(dirname "$INSTALL_PATH")"; then
        echo -e "${GREEN}Installation path is in PATH.${NC}"
    else
        echo -e "${YELLOW}Note: Add $(dirname "$INSTALL_PATH") to your PATH.${NC}"
        echo -e "${YELLOW}Add this to your ~/.bashrc or ~/.zshrc:${NC}"
        echo -e "${CYAN}export PATH=\"$(dirname "$INSTALL_PATH"):\$PATH\"${NC}"
    fi
}

install_config() {
    echo -e "${CYAN}Installing configuration...${NC}"
    
    CONFIG_DIR="$HOME/.config/rust-tool"
    mkdir -p "$CONFIG_DIR"
    
    cat > "$CONFIG_DIR/config.toml" << 'EOF'
[ai]
provider = "ollama"
model = "qwen2.5-coder:7b"
max_tokens = 4096
temperature = 0.7

[ui]
colors = true
verbose = false
yolo_mode = false

[logging]
level = "info"
format = "text"
EOF
    
    echo -e "${GREEN}Configuration installed to $CONFIG_DIR/config.toml${NC}"
}

show_usage() {
    echo ""
    echo -e "${GREEN}Installation complete!${NC}"
    echo ""
    echo -e "${CYAN}Usage:${NC}"
    echo "  code-ai                    # Start chat mode"
    echo "  code-ai -i                 # IDE mode"
    echo "  code-ai -M                 # Tool manager"
    echo "  code-ai 'Your prompt'      # Quick chat"
    echo ""
    echo -e "${YELLOW}For more options: code-ai --help${NC}"
}

main() {
    print_banner
    echo ""
    
    check_root
    echo ""
    
    check_rust
    echo ""
    
    build_project
    echo ""
    
    install_binary
    echo ""
    
    install_config
    echo ""
    
    show_usage
}

main
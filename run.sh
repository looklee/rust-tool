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
    echo "║              Unified AI Coding Assistant                  ║"
    echo "║                  All-in-One Solution                      ║"
    echo "╚═══════════════════════════════════════════════════════════╝"
    echo -e "${NC}"
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
    
    if [ -f "target/release/code" ]; then
        echo -e "${YELLOW}Found existing binary. Rebuilding...${NC}"
    fi
    
    cargo build --release -p common
    cargo build --release -p code
    
    if [ -f "target/release/code" ]; then
        echo -e "${GREEN}Build successful!${NC}"
    else
        echo -e "${RED}Build failed!${NC}"
        exit 1
    fi
}

run_code() {
    echo -e "${CYAN}Starting AI Coding Assistant...${NC}"
    echo ""
    
    if [ $# -eq 0 ]; then
        ./target/release/code
    else
        ./target/release/code "$@"
    fi
}

main() {
    print_banner
    echo ""
    
    check_rust
    echo ""
    
    build_project
    echo ""
    
    run_code "$@"
}

main "$@"
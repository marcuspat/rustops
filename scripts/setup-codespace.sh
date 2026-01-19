#!/usr/bin/env bash
#
# setup-codespace.sh - GitHub Codespace Development Environment Setup
#
# This script is idempotent (safe to run multiple times) and configures
# a GitHub Codespace for RustOps development.
#
# Usage:
#   ./scripts/setup-codespace.sh
#
# Environment variables (optional):
#   RUST_VERSION    - Rust toolchain version (default: stable)
#   SKIP_BUILD      - Set to "true" to skip pre-building (default: false)
#   VERBOSE         - Set to "true" for verbose output (default: false)
#

set -Eeuo pipefail

# ==============================================================================
# CONFIGURATION
# ==============================================================================

# Script version
SCRIPT_VERSION="1.0.0"

# Color codes for output
readonly RED='\033[0;31m'
readonly GREEN='\033[0;32m'
readonly YELLOW='\033[0;33m'
readonly BLUE='\033[0;34m'
readonly NC='\033[0m' # No Color

# Configuration defaults
RUST_VERSION="${RUST_VERSION:-stable}"
SKIP_BUILD="${SKIP_BUILD:-false}"
VERBOSE="${VERBOSE:-false}"

# Minimum Rust version required by workspace
MIN_RUST_VERSION="1.70"

# ==============================================================================
# UTILITY FUNCTIONS
# ==============================================================================

# Print colored message
log_info() {
    echo -e "${BLUE}[INFO]${NC} $*"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $*"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $*"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $*" >&2
}

# Verbose logging
log_debug() {
    if [[ "${VERBOSE}" == "true" ]]; then
        echo -e "${BLUE}[DEBUG]${NC} $*"
    fi
}

# Print section header
print_section() {
    local title="$1"
    echo ""
    echo -e "${BLUE}════════════════════════════════════════════════════════════════${NC}"
    echo -e "${BLUE}  ${title}${NC}"
    echo -e "${BLUE}════════════════════════════════════════════════════════════════${NC}"
}

# Check if command exists
command_exists() {
    command -v "$1" &>/dev/null
}

# Compare version strings
version_ge() {
    local current="$1"
    local required="$2"
    [[ "$(printf '%s\n' "$required" "$current" | sort -V | head -n1)" == "$required" ]]
}

# ==============================================================================
# DETECTION FUNCTIONS
# ==============================================================================

# Detect if running in Codespace
is_codespace() {
    [[ -n "${CODESPACES:-}" ]]
}

# Detect OS
detect_os() {
    case "$(uname -s)" in
        Linux*)  echo "linux" ;;
        Darwin*) echo "macos" ;;
        *)       echo "unknown" ;;
    esac
}

# ==============================================================================
# RUST TOOLCHAIN SETUP
# ==============================================================================

install_rust_toolchain() {
    print_section "Installing Rust Toolchain"

    local os_type
    os_type=$(detect_os)
    log_info "Detected OS: ${os_type}"
    log_info "Target Rust version: ${RUST_VERSION}"

    # Check if Rust is already installed
    if command_exists rustc; then
        local current_version
        current_version=$(rustc --version | awk '{print $2}')
        log_info "Found existing Rust installation: ${current_version}"

        # Check if current version meets minimum requirement
        if version_ge "${current_version}" "${MIN_RUST_VERSION}"; then
            log_success "Rust version ${current_version} meets minimum requirement (${MIN_RUST_VERSION})"

            # Update if using stable
            if [[ "${RUST_VERSION}" == "stable" ]]; then
                log_info "Updating Rust toolchain..."
                rustup update stable || log_warning "Failed to update Rust, continuing with current version"
            fi
        else
            log_warning "Rust version ${current_version} is below minimum ${MIN_RUST_VERSION}"
            log_info "Updating Rust toolchain..."
            rustup update || {
                log_error "Failed to update Rust. Please update manually:"
                log_error "  rustup update"
                return 1
            }
        fi
    else
        log_info "Rust not found. Installing..."

        if [[ "${os_type}" == "linux" ]]; then
            # Use rustup installer for Linux
            curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain "${RUST_VERSION}"

            # Source cargo environment
            # shellcheck source=/dev/null
            source "${HOME}/.cargo/env" 2>/dev/null || true
        else
            log_error "Unsupported OS for automatic Rust installation"
            return 1
        fi
    fi

    # Verify installation
    if command_exists rustc; then
        local final_version
        final_version=$(rustc --version)
        log_success "Rust toolchain installed: ${final_version}"
    else
        log_error "Rust installation verification failed"
        return 1
    fi

    # Install additional components
    log_info "Installing additional Rust components..."
    rustup component add rustfmt clippy || {
        log_warning "Failed to install some Rust components"
    }

    log_success "Rust toolchain setup complete"
}

# ==============================================================================
# SYSTEM DEPENDENCIES
# ==============================================================================

install_system_dependencies() {
    print_section "Installing System Dependencies"

    local os_type
    os_type=$(detect_os)

    if [[ "${os_type}" == "linux" ]]; then
        # Check for package manager
        if command_exists apt-get; then
            log_info "Detected Debian/Ubuntu-based system"
            install_apt_dependencies
        elif command_exists apk; then
            log_info "Detected Alpine Linux system"
            install_apk_dependencies
        else
            log_warning "Unknown package manager. Skipping system dependencies."
        fi
    else
        log_info "Skipping system dependencies on ${os_type}"
    fi

    log_success "System dependencies check complete"
}

install_apt_dependencies() {
    log_info "Updating package list..."
    sudo apt-get update -qq || log_warning "Failed to update package list"

    local packages=(
        # Build essentials
        build-essential
        pkg-config
        libssl-dev

        # Protobuf (for gRPC)
        protobuf-compiler

        # Utilities
        curl
        git
        jq
    )

    log_info "Installing packages: ${packages[*]}"

    # Install packages in a single call
    sudo DEBIAN_FRONTEND=noninteractive apt-get install -y -qq "${packages[@]}" || {
        log_warning "Some packages failed to install"
    }
}

install_apk_dependencies() {
    log_info "Updating package index..."
    apk update -q || log_warning "Failed to update package index"

    local packages=(
        # Build essentials
        build-base
        pkgconfig
        openssl-dev

        # Protobuf
        protobuf-dev

        # Utilities
        curl
        git
        jq
    )

    log_info "Installing packages: ${packages[*]}"
    apk add --no-cache "${packages[@]}" || {
        log_warning "Some packages failed to install"
    }
}

# ==============================================================================
# CARGO UTILITIES
# ==============================================================================

install_cargo_tools() {
    print_section "Installing Cargo Utilities"

    local tools=(
        "cargo-watch"
        "cargo-tarpaulin"
        "cargo-criterion"
        "cargo-audit"
        "cargo-duplicate"
    )

    for tool in "${tools[@]}"; do
        if command_exists "${tool}" 2>/dev/null || cargo install --list | grep -q "^${tool} "; then
            log_debug "${tool} already installed, skipping..."
        else
            log_info "Installing ${tool}..."
            cargo install "${tool}" --quiet || {
                log_warning "Failed to install ${tool}, continuing..."
            }
        fi
    done

    log_success "Cargo utilities setup complete"
}

# ==============================================================================
# PRE-BUILD PROJECT
# ==============================================================================

pre_build_project() {
    if [[ "${SKIP_BUILD}" == "true" ]]; then
        log_info "SKIP_BUILD is set, skipping pre-build..."
        return 0
    fi

    print_section "Pre-building Project"

    local project_root
    project_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

    log_info "Project root: ${project_root}"
    log_info "Building workspace (this may take a while)..."

    cd "${project_root}"

    # Build with cargo
    if cargo build --workspace 2>&1 | tee /tmp/build.log; then
        log_success "Pre-build completed successfully"
    else
        log_warning "Build encountered errors. Check /tmp/build.log for details."
        return 1
    fi
}

# ==============================================================================
# CONFIGURATION
# ==============================================================================

configure_environment() {
    print_section "Configuring Environment"

    # Create rust-toolchain.toml for consistent versioning
    local project_root
    project_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

    if [[ ! -f "${project_root}/rust-toolchain.toml" ]]; then
        log_info "Creating rust-toolchain.toml..."
        cat > "${project_root}/rust-toolchain.toml" <<EOF
[toolchain]
channel = "${RUST_VERSION}"
components = ["rustfmt", "clippy"]
EOF
        log_success "Created rust-toolchain.toml"
    else
        log_debug "rust-toolchain.toml already exists"
    fi

    # Set environment variables for Codespaces
    if is_codespace; then
        log_info "Configuring Codespace environment..."

        # Set default log level
        echo "export RUST_LOG=info" >> "${HOME}/.bashrc" 2>/dev/null || true

        # Add cargo bin to PATH if not already present
        if ! grep -q 'cargo/env' "${HOME}/.bashrc" 2>/dev/null; then
            echo 'source "${HOME}/.cargo/env"' >> "${HOME}/.bashrc" 2>/dev/null || true
        fi

        log_success "Codespace environment configured"
    fi
}

# ==============================================================================
# VALIDATION
# ==============================================================================

validate_setup() {
    print_section "Validating Setup"

    local failures=0

    # Check Rust installation
    if command_exists rustc; then
        local version
        version=$(rustc --version)
        log_success "✓ Rust installed: ${version}"

        # Check version requirement
        local current_version
        current_version=$(echo "${version}" | awk '{print $2}')
        if version_ge "${current_version}" "${MIN_RUST_VERSION}"; then
            log_success "✓ Version meets minimum requirement (${MIN_RUST_VERSION})"
        else
            log_error "✗ Version ${current_version} below minimum ${MIN_RUST_VERSION}"
            ((failures++))
        fi
    else
        log_error "✗ Rust not found"
        ((failures++))
    fi

    # Check Cargo
    if command_exists cargo; then
        log_success "✓ Cargo installed: $(cargo --version | head -n1)"
    else
        log_error "✗ Cargo not found"
        ((failures++))
    fi

    # Check project structure
    if [[ -f "Cargo.toml" ]]; then
        log_success "✓ Workspace Cargo.toml found"
    else
        log_error "✗ Cargo.toml not found"
        ((failures++))
    fi

    # Check if pre-built
    if [[ "${SKIP_BUILD}" != "true" ]]; then
        if [[ -d "target/debug" ]]; then
            log_success "✓ Project has been pre-built"
        else
            log_warning "⚠ Project not yet built (run 'cargo build' to complete)"
        fi
    fi

    echo ""
    if [[ ${failures} -eq 0 ]]; then
        log_success "All validation checks passed!"
        return 0
    else
        log_error "${failures} validation check(s) failed"
        return 1
    fi
}

# ==============================================================================
# SUMMARY
# ==============================================================================

print_summary() {
    print_section "Setup Complete"

    cat <<EOF
${GREEN}Your RustOps development environment is ready!${NC}

Quick Start Commands:
  ${YELLOW}cargo build --workspace${NC}           - Build all crates
  ${YELLOW}cargo test --workspace${NC}            - Run all tests
  ${YELLOW}cargo run --bin rustops-api${NC}        - Start API server
  ${YELLOW}cargo run --bin rustops-agent${NC}      - Start agent service

Useful Commands:
  ${YELLOW}cargo watch -x 'run --bin rustops-api'${NC}  - Hot reload development
  ${YELLOW}cargo test --workspace -- --nocapture${NC}    - Tests with output
  ${YELLOW}cargo clippy --workspace${NC}                  - Lint code
  ${YELLOW}cargo fmt --all${NC}                           - Format code

Resources:
  - README.md: Project overview and documentation
  - crates/:  Source code for each bounded context
  - tests/:   Integration and property-based tests

Environment Variables:
  RUST_LOG=info          - Set log level
  RUST_BACKTRACE=1       - Enable backtraces for errors

For more information, see: docs/development.md
EOF

    if is_codespace; then
        cat <<EOF

${BLUE}Codespace Tips:${NC}
  - Services are exposed via PORT forwarding
  - Pre-configured ports: 8080 (API), 8081 (Agent)
  - Access from "Ports" tab in VS Code
EOF
    fi
}

# ==============================================================================
# CLEANUP ON ERROR
# ==============================================================================

cleanup() {
    local exit_code=$?
    if [[ ${exit_code} -ne 0 ]]; then
        log_error "Setup failed with exit code ${exit_code}"
        log_info "Please check the error messages above and try again."
    fi
}

trap cleanup EXIT

# ==============================================================================
# MAIN EXECUTION
# ==============================================================================

main() {
    echo ""
    echo -e "${BLUE}╔═══════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${BLUE}║  RustOps Codespace Setup Script v${SCRIPT_VERSION}                   ║${NC}"
    echo -e "${BLUE}╚═══════════════════════════════════════════════════════════════╝${NC}"
    echo ""

    # Change to project root
    cd "$(dirname "${BASH_SOURCE[0]}")/.."

    # Run setup steps
    install_rust_toolchain
    install_system_dependencies
    install_cargo_tools
    configure_environment
    pre_build_project
    validate_setup

    # Print summary
    print_summary

    echo ""
    log_success "Setup script completed successfully!"
    echo ""
}

# Run main function
main "$@"

# Fast local Rust workflows.
#
# Run `just setup-fast` once, then use recipes such as:
#   just build-fast -p datatable
#   just check-fast -p re_view_datatable

set windows-shell := ["pwsh.exe", "-NoLogo", "-NoProfile", "-Command"]
set script-interpreter := ["pwsh.exe", "-NoLogo", "-NoProfile", "-File"]

nightly := "nightly-2026-09-10"
fast_config := 'include=[".cargo/cranelift.toml"]'

export CARGO_INCREMENTAL := "1"
export RUSTFLAGS := if os() == "windows" { "-Z threads=0 -C link-arg=/Brepro" } else { "-Z threads=0" }

# List the available recipes.
default:
    @just --list

# Install the pinned nightly compiler and Cranelift backend. Run this once.
setup-fast:
    pixi run rustup toolchain install {{ nightly }} --profile minimal --component rustc-codegen-cranelift-preview

# Run a Cargo subcommand with the shared fast-build configuration.
[private]
[script]
[windows]
cargo-fast subcommand *cargo_args:
    $sysroot = pixi run rustc +{{ nightly }} --print sysroot
    $env:CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER = Join-Path `
        $sysroot `
        "lib/rustlib/x86_64-pc-windows-msvc/bin/rust-lld.exe"

    pixi run cargo +{{ nightly }} {{ subcommand }} `
        --profile dev-fast `
        --config '{{ fast_config }}' `
        {{ cargo_args }}

[private]
[unix]
cargo-fast subcommand *cargo_args:
    pixi run cargo +{{ nightly }} {{ subcommand }} --profile dev-fast --config '{{ fast_config }}' {{ cargo_args }}

# Build with Cranelift. Additional arguments are passed to Cargo.
build-fast *cargo_args:
    just cargo-fast build {{ cargo_args }}

# Type-check with Cranelift. Additional arguments are passed to Cargo.
check-fast *cargo_args:
    just cargo-fast check {{ cargo_args }}

# Run tests with Cranelift. Additional arguments are passed to Cargo.
test-fast *cargo_args:
    just cargo-fast test {{ cargo_args }}

# Run the repository's quick lint checks. Additional arguments are passed to fast-lint.
lint-fast *lint_args:
    pixi run fast-lint {{ lint_args }}

# Run Clippy with the fast development configuration. Additional arguments are passed to Cargo.
clippy-fast *cargo_args:
    just cargo-fast clippy {{ cargo_args }}

# Build and run with Cranelift. Additional arguments are passed to Cargo.
run-fast *cargo_args:
    just cargo-fast run {{ cargo_args }}

# Dunward

## Dependencies

- [just](https://github.com/casey/just) for command runner
- lld (only needed for Linux and Windows)
  - Linux: Install `lld` and `clang` via package manager
  - Windows:
    ```text
    cargo install -f cargo-binutils
    rustup component add llvm-tools-preview
    ```

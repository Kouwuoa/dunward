# Dunward

## Dependencies

- [just](https://github.com/casey/just) for command runner
- lld (Linux or Windows)
  - Linux: Install `lld` and `clang` via package manager
  - Windows:
    `cargo install -f cargo-binutils`
    `rustup component add llvm-tools-preview`

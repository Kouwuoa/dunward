# Dunward

## Dependencies

- [just](https://github.com/casey/just) for command runner
- [lld](https://lld.llvm.org/) (only needed for Linux and Windows)
  - Linux: Install `lld` and `clang` via package manager
  - Windows:
    ```text
    cargo install -f cargo-binutils
    rustup component add llvm-tools-preview
    ```
- [trunk](https://github.com/trunk-rs/trunk) (only needed for web builds):
  ```text
  rustup target add wasm32-unknown-unknown
  cargo install wasm-bindgen-cli
  cargo install trunk
  ```

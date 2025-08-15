set windows-shell := ["powershell.exe", "-c"]
export RUST_BACKTRACE := "1"

deb:
    cargo run

rel:
    cargo run --release
    
web:
    trunk serve

build:
    cargo build

run:
    cargo run

set windows-shell := ["powershell.exe", "-c"]

deb:
    cargo run

rel:
    cargo run --release
    
web:
    trunk serve

build:
    cargo build

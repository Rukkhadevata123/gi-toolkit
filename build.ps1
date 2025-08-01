cargo fmt
cargo clippy --fix --allow-dirty --allow-staged
cargo build --release
cd ./hutao_minhook
cargo build --release --lib
cd ..
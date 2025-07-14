cargo fmt
cargo clippy --fix --allow-dirty --allow-staged
cargo build --release
cd bilibili_login
cargo build --release --lib 
cd ../hutao_minhook
cargo build --release --lib
cd ..
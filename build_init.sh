bash build_libxunil.sh
cd user/init
cargo build --target x86_64-unknown-none --release
cp ./target/x86_64-unknown-none/release/init ../../assets/init
cd ../..

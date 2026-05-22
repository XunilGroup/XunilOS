bash build_libxunil.sh
cd user/init
cargo build --target $KARCH-unknown-none --release --config profile.release.debug=true
cp ./target/$KARCH-unknown-none/release/init ../../assets/$KARCH/init
cd ../..

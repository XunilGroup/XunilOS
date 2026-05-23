bash build_libxunil.sh
cd user/apps/badapple
cargo build --target $KARCH-unknown-none --release --config profile.release.debug=true
cp ./target/$KARCH-unknown-none/release/badapple ../../../assets/$KARCH/badapple
cd ../../..

#!/bin/bash
if [[ "$1" = "init" || "$1" = "libxunil" ]]; then
    cd user/$1
else
    cd user/apps/$1
fi

cargo build --target $KARCH-unknown-none --release --config profile.release.debug=true
if [[ "$1" = "init" || "$1" = "libxunil" ]]; then
    cp ./target/$KARCH-unknown-none/release/$1 ../../assets/$KARCH/$1
else
    cp ./target/$KARCH-unknown-none/release/$1 ../../../assets/$KARCH/$1
fi

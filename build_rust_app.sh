#!/bin/bash

if [[ "$1" == "init" || "$1" == "libxunil" ]]; then
    base="user/$1"
else
    base="user/apps/$1"
fi

cd "$base" || exit 1

cargo build --target "$KARCH-unknown-none" --release \
    --config profile.release.debug=true

case "$1" in
    init)
        cp "./target/$KARCH-unknown-none/release/$1" \
           "../../assets/$KARCH/$1"
        ;;
    libxunil)
        ;;
    *)
        cp "./target/$KARCH-unknown-none/release/$1" \
           "../../../assets/$KARCH/$1"
        ;;
esac

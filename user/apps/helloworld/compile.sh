if [ "$KARCH" = "aarch64" ]; then
    CC="aarch64-linux-gnu-gcc"
    ARCH_FLAGS="-march=armv8-a+nosimd -mabi=lp64"
elif [ "$KARCH" = "x86_64" ]; then
    CC="gcc"
    ARCH_FLAGS="-m64 -mno-mmx -mno-avx"
else
    echo "Error: Unsupported KARCH: $KARCH"
    exit 1
fi

SYS_INCLUDES=$(echo | $CC -E -Wp,-v -xc /dev/null 2>&1 | sed -n '/^#include/,/^End of search/p' | sed -n 's/^ \(.*\)/\1/p')

GCC_INCLUDES=$($CC -print-file-name=include)
INCLUDE_FLAGS="-I../../libxunil/include -I$GCC_INCLUDES"
for dir in $SYS_INCLUDES; do
    INCLUDE_FLAGS="$INCLUDE_FLAGS -I$dir"
done

$CC -w -static -D__NO_INLINE__ -O3 $ARCH_FLAGS \
    -fno-stack-protector -fno-inline -fno-inline-small-functions \
    -fno-indirect-inlining -fno-builtin -fcompare-debug-second \
    -nostdlib -nostdinc \
    $INCLUDE_FLAGS \
    helloworld.c \
    -Wl,--gc-sections \
    -L../../libxunil/target/$KARCH-unknown-none/release -l:libxunil.a \
    -o ../../../assets/$KARCH/helloworld.elf

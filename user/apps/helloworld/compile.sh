GCC_INCLUDES=$(gcc -print-file-name=include)
SYS_INCLUDES=/usr/include
gcc -w -static -D__NO_INLINE__ -O0 -mno-mmx -mno-avx -fno-stack-protector -fno-inline -fno-inline-small-functions -fno-indirect-inlining -fno-builtin -fcompare-debug-second -nostdlib -nostdinc helloworld.c -Wl,--gc-sections -L../../libxunil/target/release -l:libxunil.a -I../../libxunil/include -I"$GCC_INCLUDES" -I"$SYS_INCLUDES" -o ../../../assets/helloworld.elf

gcc -nostdlib -nostdinc -static -no-pie \
    -o $1 $1.c \
    -L../target/release -l:libxunil.a
mv $1 ../../assets/$1.elf

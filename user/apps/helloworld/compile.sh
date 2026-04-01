gcc -nostdlib -nostdinc -static -static-pie \
    -o $1 $1.c \
    -L../../libxunil/target/release -l:libxunil.a
mv $1 ../../../assets/$1.elf

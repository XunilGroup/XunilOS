bash build_libxunil.sh
cd user/apps/doomgeneric/doomgeneric
rm -r ./build
make -f Makefile.xunil
cp doomgeneric ../../../../assets/doomgeneric
cd ../../../..

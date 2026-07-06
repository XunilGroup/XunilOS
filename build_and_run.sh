export KARCH=x86_64
mkdir -p assets/aarch64
mkdir -p assets/x86_64
bash build_all.sh
make run

void write(int fd, const char* buf, unsigned long count);
void exit(int code);

void _start() {
    write(1, "Hello from C!\n", 14);
    exit(0);
}

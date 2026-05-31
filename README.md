# XunilOS
XunilOS is an OS made from scratch in Rust.
The repo is based on the limine-rust-template.

It supports aarch64 inside QEMU, and x86_64 even on bare machines!

# Features

## Kernel
- x86_64 IDT, GDT, interrupts, kernel heap, PS2 mouse/keyboard, paging, syscalls and usermode.
- aarch64 kernel heap, paging, interrupts, Virtio mouse/keyboard, syscalls, usermode
- Scheduler which does round-robin switching as well as sleeping and waking processes
- ELF64 ET_EXEC and ET_DYN loading, verifying and running support
- A readonly VFS which currently includes the ELF files which can be ran
- IPC with granular permissions (read, write, manage)
- basic (and insecure) SHM support
- Limine bootloader
- Framebuffer and serial support
- Timing support based on IRQ
- Per-process address space, kernel & user stack
- Copy to and from userspace

## Apps
- doomgeneric: Doom ported to XunilOS. Isn't as easy, as i am using Rust and had to write my own libc stub.
- badapple: Grayscale Bad Apple by using numbers to represent shades of gray. Pretty easy, but this is the only place where I also used Python.
- shell: simple shell with elf running, echo and file read commands.
- helloworld: just prints helloworld to serial

## Init
- Desktop-like experience
- Window Management (close and minimize)
- Start menu to open applications
- BMP background
- Mouse support with a BMP image
- Dock where you can see currently open applications which can be minimized or unminimized

## Libxunil (libc stub)
- Basic functions of C (printf, strlen, etc)
- File I\O, IPC, time
- Input reading from Window Management
- SHM
- User Heap
- Window management
- IPC support
- Syscalls to call back to the kernel
- Primitives, Framebuffer and font rendering

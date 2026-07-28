# XunilOS
XunilOS is an OS made from scratch in Rust.
The repo is based on the limine-rust-template.

It supports aarch64 inside QEMU, and x86_64 even on bare machines!

## IMPORTANT
For some reason, it doesn't work with default QEMU or VirtualBox settings.
So, you have to:
```
qemu-system-x86_64 -M q35 -serial stdio -cdrom XunilOS-x86_64.iso -m 4G
```
Or:
```
qemu-system-aarch64 -M virt,gic-version=2,secure=off -cpu cortex-a72 -device ramfb -device qemu-xhci -device usb-kbd -device usb-mouse -device virtio-keyboard-device -device virtio-mouse-device -serial stdio -cdrom XunilOS-aarch64.iso -semihosting-config enable=on,target=native -m 4G -global virtio-mmio.force-legacy=false
```

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

## Mirrors

[![Forgejo](https://img.shields.io/badge/Forgejo-git.csd4ni3l.hu-1e90ff)](https://git.csd4ni3l.hu/XunilGroup/XunilOS)
[![GitHub](https://img.shields.io/badge/GitHub-github.com-181717)](https://github.com/XunilGroup/XunilOS)
[![Codeberg](https://img.shields.io/badge/Codeberg-codeberg.org-2185D0)](https://codeberg.org/XunilGroup/XunilOS)

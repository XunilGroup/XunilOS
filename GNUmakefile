include config.mk

# Nuke built-in rules and variables.
MAKEFLAGS += -rR
.SUFFIXES:

override QEMUFLAGS := -m $(MEMORY) -global virtio-mmio.force-legacy=false
override IMAGE_NAME := XunilOS-$(KARCH)

.PHONY: all
all: $(IMAGE_NAME).iso

.PHONY: run
run: run-$(KARCH)

.PHONY: run-x86_64
run-x86_64: edk2-ovmf $(IMAGE_NAME).iso
	qemu-system-$(KARCH) \
		-M q35 \
		-serial stdio \
		-drive if=pflash,unit=0,format=raw,file=edk2-ovmf/ovmf-code-$(KARCH).fd,readonly=on \
		-cdrom $(IMAGE_NAME).iso \
		$(QEMUFLAGS)

.PHONY: debug-x86_64
debug-x86_64: edk2-ovmf $(IMAGE_NAME).iso
	qemu-system-$(KARCH) \
		-M q35 \
		-serial stdio \
		-drive if=pflash,unit=0,format=raw,file=edk2-ovmf/ovmf-code-$(KARCH).fd,readonly=on \
		-cdrom $(IMAGE_NAME).iso \
		-d in_asm,int,mmu -dfilter 0xffffffff80000000..0xffffffff80010000 -D /tmp/qemu_trace.log 2>/dev/null \
		$(QEMUFLAGS)

.PHONY: run-aarch64
run-aarch64: edk2-ovmf $(IMAGE_NAME).iso
	qemu-system-$(KARCH) \
		-M virt,gic-version=2,secure=off \
		-cpu cortex-a72 \
		-device ramfb \
		-device qemu-xhci \
		-device usb-kbd \
		-device usb-mouse \
		-device virtio-keyboard-device \
		-device virtio-mouse-device \
		-serial stdio \
		-drive if=pflash,unit=0,format=raw,file=edk2-ovmf/ovmf-code-$(KARCH).fd,readonly=on \
		-cdrom $(IMAGE_NAME).iso \
		-semihosting-config enable=on,target=native \
		$(QEMUFLAGS)

.PHONY: debug-aarch64
debug-aarch64: edk2-ovmf $(IMAGE_NAME).iso
	qemu-system-$(KARCH) \
		-M virt,gic-version=2,secure=off \
		-cpu cortex-a72 \
		-device ramfb \
		-device qemu-xhci \
		-device usb-kbd \
		-device usb-mouse \
		-device virtio-keyboard-device \
		-device virtio-mouse-device \
		-serial stdio \
		-drive if=pflash,unit=0,format=raw,file=edk2-ovmf/ovmf-code-$(KARCH).fd,readonly=on \
		-cdrom $(IMAGE_NAME).iso \
		-semihosting-config enable=on,target=native \
		-d in_asm,int,mmu -dfilter 0xffffffff80000000..0xffffffff80010000 -D /tmp/qemu_trace.log 2>/dev/null \
		$(QEMUFLAGS)

edk2-ovmf:
		curl -L https://github.com/osdev0/edk2-ovmf-nightly/releases/latest/download/edk2-ovmf.tar.gz | gunzip | tar -xf -

limine/limine:
	rm -rf limine
	git clone https://github.com/limine-bootloader/limine.git --branch=v10.x-binary --depth=1
	$(MAKE) -C limine

.PHONY: kernel
kernel:
	$(MAKE) -C kernel

$(IMAGE_NAME).iso: limine/limine kernel
	rm -rf iso_root
	mkdir -p iso_root/boot
	cp -v kernel/kernel iso_root/boot/
	mkdir -p iso_root/boot/limine
	cp -v limine.conf iso_root/boot/limine/
	mkdir -p iso_root/EFI/BOOT
ifeq ($(KARCH),x86_64)
	cp -v limine/limine-bios.sys limine/limine-bios-cd.bin limine/limine-uefi-cd.bin iso_root/boot/limine/
	cp -v limine/BOOTX64.EFI iso_root/EFI/BOOT/
	cp -v limine/BOOTIA32.EFI iso_root/EFI/BOOT/
	xorriso -as mkisofs -b boot/limine/limine-bios-cd.bin \
		-no-emul-boot -boot-load-size 4 -boot-info-table \
		--efi-boot boot/limine/limine-uefi-cd.bin \
		-efi-boot-part --efi-boot-image --protective-msdos-label \
		iso_root -o $(IMAGE_NAME).iso
	./limine/limine bios-install $(IMAGE_NAME).iso
endif
ifeq ($(KARCH),aarch64)
	cp -v limine/limine-uefi-cd.bin iso_root/boot/limine/
	cp -v limine/BOOTAA64.EFI iso_root/EFI/BOOT/
	xorriso -as mkisofs \
		--efi-boot boot/limine/limine-uefi-cd.bin \
		-efi-boot-part --efi-boot-image --protective-msdos-label \
		iso_root -o $(IMAGE_NAME).iso
endif
ifeq ($(KARCH),riscv64)
	cp -v limine/limine-uefi-cd.bin iso_root/boot/limine/
	cp -v limine/BOOTRISCV64.EFI iso_root/EFI/BOOT/
	xorriso -as mkisofs \
		--efi-boot boot/limine/limine-uefi-cd.bin \
		-efi-boot-part --efi-boot-image --protective-msdos-label \
		iso_root -o $(IMAGE_NAME).iso
endif
ifeq ($(KARCH),loongarch64)
	cp -v limine/limine-uefi-cd.bin iso_root/boot/limine/
	cp -v limine/BOOTLOONGARCH64.EFI iso_root/EFI/BOOT/
	xorriso -as mkisofs \
		--efi-boot boot/limine/limine-uefi-cd.bin \
		-efi-boot-part --efi-boot-image --protective-msdos-label \
		iso_root -o $(IMAGE_NAME).iso
endif
	rm -rf iso_root

.PHONY: clean
clean:
	$(MAKE) -C kernel clean
	rm -rf iso_root $(IMAGE_NAME).iso

.PHONY: distclean
distclean: clean
	$(MAKE) -C kernel distclean
	rm -rf limine ovmf

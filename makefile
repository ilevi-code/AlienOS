TOOLCHAIN := arm-none-eabi-
CC := $(TOOLCHAIN)gcc
AS := $(TOOLCHAIN)as
LD := $(TOOLCHAIN)ld
OBJCOPY := $(TOOLCHAIN)objcopy
ASFLAGS := -g -march=armv7-a
CFLAGS := -g -nostdlib -nostdinc -march=armv7-a

KERNEL ?= target/armv7a-none-eabi/debug/alienos

ifneq ($(QEMU_DEBUG),)
QEMU_FLAGS := -S -s
endif

.PHONY: clean init

qemu: kernel.bin | fs.img
	qemu-system-arm -m 512M -M virt $(QEMU_FLAGS) -semihosting -nographic -kernel kernel.bin \
		-global virtio-mmio.force-legacy=false \
		-drive file=fs.img,format=raw,if=none,id=hd0 \
		-device virtio-blk-device,drive=hd0 \
		-device virtio-net-device,netdev=net0 -netdev user,id=net0

kernel.bin: $(KERNEL) boot.elf FORCE
	$(OBJCOPY) --only-keep-debug $< debug.elf
	$(OBJCOPY) -O binary $< _kernel.bin
	$(OBJCOPY) -O binary boot.elf _boot.bin
	cat _boot.bin _kernel.bin > $@

FORCE: ;

boot.elf: boot.o bootstrap_mmu.o mmu.o
	$(LD) -T boot.ld $^ -o $@

clean:
	$(RM)  *.bin *.elf *.o

fs.img: init
	touch $@
	truncate -s 1M $@
	echo y | mkfs.ext2 -I 128 -t small -O ^sparse_super,^large_file,^resize_inode,^dir_index,^ext_attr fs.img
	e2mkdir fs.img:/sbin
	e2cp init/target/armv7a-none-eabi/release/init fs.img:/sbin/init

init:
	cd init && cargo build --release

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

.PHONY: clean init loader

qemu: build/kernel.bin | build/fs.img
	qemu-system-arm -m 512M -M virt $(QEMU_FLAGS) -semihosting -nographic \
		-kernel build/kernel.bin \
		-global virtio-mmio.force-legacy=false \
		-drive file=build/fs.img,format=raw,if=none,id=hd0 \
		-device virtio-blk-device,drive=hd0 \
		-device virtio-net-device,netdev=net0 -netdev user,id=net0

build/kernel.bin: $(KERNEL) loader/loader.elf FORCE | build
	$(OBJCOPY) --only-keep-debug $< build/debug.elf
	$(OBJCOPY) -O binary $< build/_kernel.bin
	$(OBJCOPY) -O binary loader/loader.elf build/_boot.bin
	cat build/_boot.bin build/_kernel.bin > $@

FORCE: ;

loader/boot.elf:
	make -C loader LD=$(LD) AS=$(AS) CC=$(CC)

clean:
	$(RM)  *.bin *.elf *.o
	make -C loader clean

build/fs.img: init | build
	touch $@
	truncate -s 1M $@
	echo y | mkfs.ext2 -I 128 -t small -O ^sparse_super,^large_file,^resize_inode,^dir_index,^ext_attr $@
	e2mkdir $@:/sbin
	e2cp init/target/armv7a-none-eabi/release/init $@:/sbin/init

init:
	cd init && cargo build --release

build:
	mkdir -p $@

TOOLCHAIN := arm-none-eabi-
CC := $(TOOLCHAIN)gcc
AS := $(TOOLCHAIN)as
LD := $(TOOLCHAIN)ld
OBJCOPY := $(TOOLCHAIN)objcopy
ASFLAGS := -g -march=armv7-a
CFLAGS := -g -nostdlib -nostdinc -march=armv7-a

.PHONY: clean

ifneq ($(QEMU_DEBUG),)
QEMU_FLAGS := -S -s
endif

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

fs.img:
	touch $@
	truncate -s 1024 $@


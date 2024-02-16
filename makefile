TOOLCHAIN := arm-none-eabi-
CC := $(TOOLCHAIN)gcc
AS := $(TOOLCHAIN)as
ASFLAGS := -g
CFLAGS := -g -nostdlib -nostdinc

SRC_DIR := .

# CFILES := $(wildcard $(SRC_DIR)/*.c)
CFILES := main.c console.c virtio.c mmu.c
# ASMFILES := $(wildcard $(SRC_DIR)/*.s)
ASMFILES := interrupt_vectors.s
OBJFILES := $(patsubst %.c,%.o,$(CFILES)) $(patsubst %.s,%.o,$(ASMFILES))

.PHONY: clean run

kernel.bin: kernel.elf boot.elf
	$(TOOLCHAIN)objcopy -O binary kernel.elf _kernel.bin
	$(TOOLCHAIN)objcopy -O binary boot.elf _boot.bin
	cat _boot.bin _kernel.bin > $@

kernel.elf: $(OBJFILES)
	$(TOOLCHAIN)ld -T kernel.ld $^ -o $@

boot.elf: boot.o bootstrap_mmu.o mmu.o
	$(TOOLCHAIN)ld -T boot.ld $^ -o $@

clean:
	$(RM)  *.bin *.elf *.o

qemu: kernel.bin | fs.img
	qemu-system-arm -m 512M -M virt -S -s -nographic -kernel kernel.bin \
		-global virtio-mmio.force-legacy=false \
		-drive file=fs.img,format=raw,if=none,id=hd0 \
		-device virtio-blk-device,drive=hd0 \
		-device virtio-net-device,netdev=net0 -netdev user,id=net0

fs.img:
	touch $@
	truncate -s 1024 $@


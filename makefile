TOOLCHAIN := arm-none-eabi-
CC := $(TOOLCHAIN)gcc
AS := $(TOOLCHAIN)as
ASFLAGS := -g
CFLAGS := -g

SRC_DIR := .

# CFILES := $(wildcard $(SRC_DIR)/*.c)
CFILES := main.c console.c virtio.c mmu.c
# ASMFILES := $(wildcard $(SRC_DIR)/*.s)
ASMFILES := startup.s
OBJFILES := $(patsubst %.c,%.o,$(CFILES)) $(patsubst %.s,%.o,$(ASMFILES))

.PHONY: clean run

main.bin: main.elf
	$(TOOLCHAIN)objcopy -O binary $^ $@

main.elf: $(OBJFILES)
	$(TOOLCHAIN)ld -T main.ld $^ -o $@

clean:
	$(RM)  *.bin *.elf *.o

qemu: main.bin | fs.img
	qemu-system-arm -m 512M -M virt -s -nographic -kernel main.bin \
		-global virtio-mmio.force-legacy=false \
		-drive file=fs.img,format=raw,if=none,id=hd0 \
		-device virtio-blk-device,drive=hd0 \
		-device virtio-net-device,netdev=net0 -netdev user,id=net0

fs.img:
	touch $@
	truncate -s 1024 $@


.PHONY: clean

loader.elf: loader.o bootstrap_mmu.o mmu.o
	$(LD) -T loader.ld $^ -o $@

clean:
	$(RM) *.o

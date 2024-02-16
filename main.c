#include "console.h"
#include "mmu.h"

extern char kernel_end[];

void kernel_entry(void* dtb) {
    write_uart0("mmu on\n");

    panic("end");
}

#include "console.h"
#include "mmu.h"

extern char kernel_end[];

void c_entry() {
    mmu_init();
    write_uart0("mmu on\n");

    panic("end");
}

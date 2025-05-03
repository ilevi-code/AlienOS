#include "mmu.h"
#include "utils.h"

extern uint8_t __start;
extern uint8_t __end;
extern translation_table_t bootstrap_table;

void map_sections(translation_table_t* table, uint32_t va, uint32_t pa, uint32_t size, uint32_t flags)
{
    int table_index = va / SECTION_SIZE;
    int section_count = CEIL_DIV(size, SECTION_SIZE);
    for (int i = 0; i < section_count; i++, table_index++) {
        table->entries[table_index] = (pa + (SECTION_SIZE * i)) | TT_ENTRY_SECTION | flags;
    }
}

void mmu_on()
{
    uint32_t status;
    // NOPs are used to prevent execution of pre-fetched instructions
    asm volatile("MRC p15, 0, %0, c1, c0, 0\n"
                 "ORR %0, %0, #0x1\n"
                 "MCR p15, 0, %0, c1, c0, 0\n"
                 "NOP\n"
                 "NOP\n"
                 : "=r"(status));
}

translation_table_t* mmu_init()
{
    uint32_t bootstrap_size = (&__end - &__start);
    uint32_t start = (uint32_t)&__start;

    for (uint32_t i = 0; i < TRANSLATION_TABLE_ENTRIES; i++) {
        bootstrap_table.entries[i] = 0;
    }

    // Map MMIO
    map_sections(&bootstrap_table, 0, 0, 0x40000000, SECTION_AP(PERM_NONE) | TT_ENTRY_B);

    // Use 1:1 mapping for the the bootstrap code
    map_sections(&bootstrap_table, start, start, bootstrap_size, SECTION_AP(PERM_NONE));

    // Map the kernel to the higher 1GB
    map_sections(&bootstrap_table, 0xc0000000, 0x40000000, 0x10000000, SECTION_AP(PERM_NONE));
    map_sections(&bootstrap_table, 0xfff00000, 0x48000000, SECTION_SIZE, SECTION_AP(PERM_NONE));

    set_ttbr0(&bootstrap_table);

    // set all domains as "clients". This means that the permission bits in the
    // translation table are checked upon access.
    set_dacr(0x77777777);

    mmu_on();

    return &bootstrap_table;
};

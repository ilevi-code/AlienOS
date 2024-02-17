#include "mmu.h"

#define TRANSLATION_TABLE_ALIGN(table) (((uint32_t)(table)) & ~0x3fff)

inline uint32_t get_ttbr0()
{
    uint32_t ttbr;
    asm volatile("MRC p15, 0, %0, c2, c0, 0" : "=r"(ttbr));
    return ttbr;
}

void set_ttbr0(translation_table_t* table)
{
    asm volatile("MCR p15, 0, %0, c2, c0, 0" : : "r"(table));
}

void set_ttbr1(translation_table_t* table)
{
    asm volatile("MCR p15, 0, %0, c2, c0, 1" : : "r"(table));
}

void set_dacr(uint dacr)
{
    asm volatile("MCR p15, 0, %0, c3, c0, 0" : : "r"(dacr));
}

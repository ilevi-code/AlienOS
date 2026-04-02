#include "mmu.h"

#define TRANSLATION_TABLE_ALIGN(table) (((uint32_t)(table)) & ~0x3fff)

inline uint32_t get_ttbr0()
{
    uint32_t ttbr;
    asm volatile("MRC p15, 0, %0, c2, c0, 0" : "=r"(ttbr));
    return ttbr;
}

void set_ttbr0(void* table)
{
    asm volatile("MCR p15, 0, %0, c2, c0, 0" : : "r"(table));
}

void set_ttbr1(void* table)
{
    asm volatile("MCR p15, 0, %0, c2, c0, 1" : : "r"(table));
}

uint32_t get_ttbcr()
{
    uint32_t ttbcr;
    asm volatile("MRC p15, 0, %0, c2, c0, 2" : "=r"(ttbcr));
}

void set_ttbcr(uint32_t ttbcr)
{
    asm volatile("MCR p15, 0, %0, c2, c0, 2" : : "r"(ttbcr));
}

void set_dacr(uint dacr)
{
    asm volatile("MCR p15, 0, %0, c3, c0, 0" : : "r"(dacr));
}

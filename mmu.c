#include "mmu.h"
#include "utils.h"

uint32_t get_ttbr0()
{
    uint32_t ttbr;
    asm volatile("MRC p15, 0, %0, c2, c0, 0" : "=r"(ttbr));
    return ttbr;
}

#define TRANSLATION_TABLE_ALIGN(table) (((uint32_t)(table)) & ~0x3fff)

void set_ttbr0(translation_table_t* table)
{
    /* check that ttbr is aligned: (ttbr & ~0x3fff) == ttbr */
    asm volatile("MCR p15, 0, %0, c2, c0, 0" : : "r"(table));
}

void set_ttbr1(translation_table_t* table)
{
    // ttbr &= ~0x3fff;
    asm volatile("MCR p15, 0, %0, c2, c0, 1" : : "r"(table));
}

#define TT_ENTRY_L2_TABLE (0x1)
#define TT_ENTRY_SECTION (0x2)
#define L2_ENTRY_LARGE_PAGE (0x1)
#define L2_ENTRY_SMALL_PAGE (0x2)

// for all but PERM_PRIV_NONE, priveleged access allows RW
#define PERM_PRIV_NONE (0)
#define PERM_NONE (1)
#define PERM_RO (2)
#define PERM_RW (3)
// set the access-permission per page/section
#define SECTION_AP(perm) ((perm) << 10)
#define PAGE_AP(perm) ((perm) << 4)

// TEX, B and C flags provide different memory models
// TEX=0, B=0, C=0 -> Strongly-ordered memory
// TEX=0, B=0, C=1 -> Device (MMIO)
// TEX=0, B=1, C=0 -> Write-through
// TEX=0, B=1, C=1 -> Write-back
// TEX=1, B=0, C=0 -> No-cache
#define TT_ENTRY_B (1 << 2)
#define TT_ENTRY_C (1 << 3)
#define SECTION_TEX(tex) ((tex) << 12)
#define SMALL_PAGE_TEX(tex) ((tex) << 12)
#define LARGE_PAGE_TEX(tex) ((tex) << 6)

// never-execute bit
#define SECTION_NX (1 << 4)
#define LARGE_PAGE_NX (1 << 15)
#define SMALL_PAGE_NX (1 << 0)

#define SECTION_DOMAIN(domain_num) ((domain_num) << 5)
#define L2_DOMAIN(domain_num) ((domain_num) << 5)

void set_dacr(uint dacr)
{
    asm volatile("MCR p15, 0, %0, c3, c0, 0" : : "r"(dacr));
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

translation_table_t bootstrap_map;

void map_sections(translation_table_t* table, uint32_t va, uint32_t pa, uint32_t size, uint32_t flags)
{
    int start_index = va / SECTION_SIZE;
    int end_index = CEIL_DIV(va + size, SECTION_SIZE);
    for (int i = start_index; i < end_index; i++) {
        table->entries[i] = (SECTION_SIZE * i) | TT_ENTRY_SECTION | flags;
    }
}

void mmu_init()
{
    // Use 1:1 mapping for the first 2GB.
    // The first 1G is MMIO, so set the B bit (as per ARMv7-A docs)
    map_sections(&bootstrap_map, 0, 0, 0x40000000, SECTION_AP(PERM_NONE) | TT_ENTRY_B);
    map_sections(&bootstrap_map, 0x40000000, 0x40000000, 0x80000000, SECTION_AP(PERM_NONE));
    set_ttbr0(&bootstrap_map);

    // set all domains as "clients". This means that the permission bits in the
    // translation table are checked upon access.
    set_dacr(0x77777777);

    mmu_on();
};

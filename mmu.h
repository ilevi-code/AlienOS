#pragma once

#include "types.h"

// A virtual address has a three-part structure as follows:
//
// +-------12------+-------8--------+---------12----------+
// | Level 1 table | Level 2 table  | Offset within Page  |
// |      Index    |      Index     |                     |
// +---------------+----------------+---------------------+

// Arm allows 1MB section mapping
//
// +-------12------+----------------20--------------------+
// | Level 1 table |        Offset within Section         |
// |      Index    |                                      |
// +---------------+--------------------------------------+

#define TRANSLATION_TABLE_ENTRIES 4096
#define L2_TABLE_ENTRIES 256

#define SECTION_SIZE (1024 * 1024)
#define SMALL_PAGE_SIZE (4096)
#define LARGE_PAGE_SIZE (65536)

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


typedef enum
{
    PAGING_16KB = 0,
    PAGING_8KB = 1,
    PAGING_4KB = 2,
} paging_size;

typedef struct
{
    uint8_t page[0x1000];
} small_page_t;

typedef struct
{
    uint8_t page[0x64000];
} large_page_t;

typedef struct
{
    uint8_t section[SECTION_SIZE];
} section_t;

typedef union
{
    large_page_t* large;
    small_page_t* small;
} l2_entry_t;

typedef struct
{
    l2_entry_t entries[L2_TABLE_ENTRIES];
} l2_translation_table_t;

typedef struct
{
    union
    {
        l2_translation_table_t* l2[TRANSLATION_TABLE_ENTRIES];
        section_t* sections[TRANSLATION_TABLE_ENTRIES];
        uint32_t entries[TRANSLATION_TABLE_ENTRIES];
    };
} translation_table_t;

void set_ttcr(uint32_t table);

void set_dacr(uint dacr);

void set_ttbr1(translation_table_t* table);

void set_ttbr0(translation_table_t* table);

void mmu_init();

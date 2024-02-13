#pragma once

#include "types.h"

// A virtual address has a three-part structure as follows:
//
// +--------12------+-------8--------+---------12----------+
// | Page Directory |   Page Table   | Offset within Page  |
// |      Index     |      Index     |                     |
// +----------------+----------------+---------------------+

// Arm allows 1MB mapping
//
// +--------12------+----------------20--------------------+
// | Page Directory |        Offset within Section         |
// |      Index     |                                      |
// +----------------+--------------------------------------+

#define TRANSLATION_TABLE_ENTRIES 4096
#define L2_TABLE_ENTRIES 256

#define SECTION_SIZE (1024 * 1024)

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
} translation_table_t __attribute__((aligned(0x4000)));

void set_ttcr(uint32_t table);

void set_ttbr1(translation_table_t* table);

void set_ttbr0(translation_table_t* table);

void mmu_init();

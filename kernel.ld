ENTRY(kernel_entry)
PHDRS {
    text PT_LOAD ;
}
SECTIONS
{
 . = 0xc0018000;
 kernel_start = .;
 entry : { kernel_entry.o(entry) }
 interrupt_table ALIGN(0x40): { interrupt_table.o(interrupt_table) }
 .text : { *(.text*) }
 .data : { *(.data*) }
 .bss : { *(.bss* COMMON) }
 . = ALIGN(0x4000);
 kernel_end = .;
}

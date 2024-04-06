ENTRY(kernel_entry)
PHDRS {
    text PT_LOAD ;
}
SECTIONS
{
 . = 0xc0018000;
 kernel_start = .;
 .entry : { kernel_entry.o(.text) } : text
 /* todo - check why .text* changes the entrypoint */
 .text : { *(.text) }
 .data : { *(.data*) }
 .bss : { *(.bss* COMMON) }
 . = ALIGN(0x4000);
 kernel_end = .;
}

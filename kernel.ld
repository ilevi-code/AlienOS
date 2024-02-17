ENTRY(kernel_entry)
PHDRS {
    text PT_LOAD ;
}
SECTIONS
{
 . = 0xc0018000;
 .entry : { entry.o(.text) } : text
 .text : { *(.text) }
 .data : { *(.data) }
 .bss : { *(.bss COMMON) }
 . = ALIGN(0x4000);
 kernel_end = .;
}

ENTRY(kernel_entry)
SECTIONS
{
 . = 0xc0018000;
 .entry : { main.o(.text) }
 .text : { *(.text) }
 .data : { *(.data) }
 .bss : { *(.bss COMMON) }
 . = ALIGN(0x4000);
 kernel_end = .;
}

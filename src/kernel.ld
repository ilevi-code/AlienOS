ENTRY(kernel_entry)
PHDRS {
    text PT_LOAD ;
}
SECTIONS
{
 . = 0x80018000;
 kernel_start = .;
 entry : { kernel_entry.o(entry) }
 interrupt_table ALIGN(0x40): { interrupt_table.o(interrupt_table) }
 .text : { *(.text*) }
 .data : {
    *(.data*)

    . = ALIGN(4);
    __faults_start = .;
    *(faults)
    __faults_end = .;

    . = ALIGN(4);
    __syscalls_start = .;
    *(syscalls)
    __syscalls_end = .;
 }
 .bss : { *(.bss* COMMON) }
 . = ALIGN(0x4000);
 kernel_end = .;
}

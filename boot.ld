ENTRY(_reset)
SECTIONS
{
 . = 0x40010000;
 __start = .;
 .text : { *(.text) }
 .data : {
     . = . + 0x1000; /* 4KB of stack space */
     init_stack = .;
     /* align bootstrap translation table to 16KB */
     . = ALIGN(0x4000);
     bootstrap_table = .;
     . = . + 0x4000;
 } = 0
 __end = .;
 .asserts : {
     ASSERT (__end == 0x40018000, "Bad end address of boot image, will mismatch kernel.ld");
 }
 /* Since we map ourselves to the higher 1GB of the RAM, we
  * need to jump to the correct address */
 kernel_entry = 0x80018000;
}

.section entry, "ax"
.global kernel_entry
.type kernel_entry,%function
.cfi_startproc
kernel_entry:
add	sp, #0x40000000
add	r2, #0x40000000
BL main
.cfi_endproc


.section entry, "ax"
.global kernel_entry
.type kernel_entry,%function
.cfi_startproc
kernel_entry:
add	sp, #-2147483648
BL main
.cfi_endproc


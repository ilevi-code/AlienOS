.text
.global kernel_entry
.type kernel_entry,%function
.cfi_startproc
kernel_entry:
BL main
.cfi_endproc


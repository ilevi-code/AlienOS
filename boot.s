.text
.global _reset
_reset:
    LDR sp, =init_stack
    // QEMU stores a pointer to the DTB in r2
    PUSH {r2}
    BL mmu_init
    MOV r1, r0
    POP {r0}
    LDR r2, =init_stack
    BL kernel_entry
end:
    B end

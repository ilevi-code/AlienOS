.global _reset
_reset:
    LDR sp, =stack_top
    BL c_entry
end:
    B end

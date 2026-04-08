#[macro_export]
macro_rules! volatile_reg_read {
    ($field:tt) => {
        #[inline]
        pub fn $field(&self) -> u32 {
            // Safety:
            // regs are MMIO
            unsafe { core::ptr::addr_of!(self.$field).read_volatile() }
        }
    };
}

#[macro_export]
macro_rules! volatile_reg_write {
    ($field:tt) => {
        paste::paste! {
            #[inline]
            pub fn [< set_ $field >] (&mut self, value: u32) {
                // Safety:
                // regs are MMIO
                unsafe { core::ptr::addr_of_mut!(self.$field).write_volatile(value) }
            }
        }
    };
}

#[macro_export]
macro_rules! volatile_reg {
    ($field:tt) => {
        $crate::volatile_reg_read!($field);
        $crate::volatile_reg_write!($field);
    };
}

#[macro_export]
macro_rules! volatile_reg_cell_read {
    ($field:tt) => {
        #[inline]
        pub fn $field(&self) -> u32 {
            // Safety:
            // regs are MMIO
            unsafe { self.$field.get().read_volatile() }
        }
    };
}

#[macro_export]
macro_rules! volatile_reg_cell_write {
    ($field:tt) => {
        paste::paste! {
            #[inline]
            pub fn [< set_ $field >] (&self, value: u32) {
                // Safety:
                // regs are MMIO
                unsafe { self.$field.get().write_volatile(value) }
            }
        }
    };
}

#[macro_export]
macro_rules! volatile_reg_cell {
    ($field:tt) => {
        $crate::volatile_reg_cell_read!($field);
        $crate::volatile_reg_cell_write!($field);
    };
}

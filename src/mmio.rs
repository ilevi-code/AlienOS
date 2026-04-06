#[macro_export]
macro_rules! volatile_reg_read {
    ($field:tt) => {
        #[inline]
        pub fn $field(&self) -> u32 {
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
                unsafe { core::ptr::addr_of_mut!(self.$field).write_volatile(value) }
            }
        }
    };
}

#[macro_export]
macro_rules! volatile_reg {
    ($field:tt) => {
        volatile_reg_read!($field);
        volatile_reg_write!($field);
    };
}

pub(crate) use device_tree::DeviceTree;
pub(crate) use memory::Memory;

mod bytes_reader;
mod consume;
mod device_tree;
mod error;
mod flattened_header;
mod interrupt_controller;
mod interrupts;
mod memory;
mod parse;
mod pl011;
mod root;
mod string_block;
mod timer;
mod tokens;
mod clock;

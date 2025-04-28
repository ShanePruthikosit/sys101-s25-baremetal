#[global_allocator]
static ALLOCATOR: DummyAllocator = DummyAllocator;

use alloc::alloc::{GlobalAlloc, Layout};
use core::ptr::null_mut;
use core::fmt::Write;

use crate::serial;
pub struct DummyAllocator;

pub static mut HEAP_START: usize = 0x0;
pub static mut OFFSET: usize = 0x0;
pub const HEAP_SIZE: usize = 100 * 1024; // 100 KiB

fn align_up(addr: usize, align: usize) -> usize {
    (addr + align - 1) & !(align - 1)
}

unsafe impl GlobalAlloc for DummyAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let size = layout.size();
        let align = layout.align();

        let aligned_offset = align_up(HEAP_START + OFFSET, align);
        let new_offset = aligned_offset + size;

        if new_offset - HEAP_START > HEAP_SIZE {
            writeln!(serial(), "alloc failed: not enough memory").ok();
            return null_mut();
        }

        let ptr = aligned_offset as *mut u8;
        OFFSET = new_offset - HEAP_START;
        ptr
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        writeln!(serial(), "dealloc was called at {_ptr:?}").unwrap();
    }
}

pub fn init_heap(offset: usize) {
    unsafe {
        HEAP_START = offset;
        OFFSET = 0;
    }
}
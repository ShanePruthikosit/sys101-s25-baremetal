#![feature(sync_unsafe_cell)]
#![feature(abi_x86_interrupt)]
#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points

extern crate alloc;

mod screen;
mod allocator;
mod frame_allocator;
mod interrupts;
mod gdt;
mod pong;

use alloc::boxed::Box;
use core::fmt::Write;
use core::slice;
use core::sync::atomic::{AtomicBool, Ordering};
use bootloader_api::{entry_point, BootInfo, BootloaderConfig};
use bootloader_api::config::Mapping::Dynamic;
use bootloader_api::info::MemoryRegionKind;
use kernel::{HandlerTable, serial};
use pc_keyboard::{DecodedKey, KeyCode};
use x86_64::registers::control::Cr3;
use x86_64::VirtAddr;
use crate::frame_allocator::BootInfoFrameAllocator;
use crate::screen::{Writer, screenwriter};

// Track key states locally
static KEY_W_ACTIVE: AtomicBool = AtomicBool::new(false);
static KEY_S_ACTIVE: AtomicBool = AtomicBool::new(false);

const BOOTLOADER_CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    config.mappings.physical_memory = Some(Dynamic); // obtain physical memory offset
    config.kernel_stack_size = 256 * 1024; // 256 KiB kernel stack size
    config
};
entry_point!(kernel_main, config = &BOOTLOADER_CONFIG);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    writeln!(serial(), "Entered kernel with boot info: {boot_info:?}").unwrap();
    writeln!(serial(), "Frame Buffer: {:p}", boot_info.framebuffer.as_ref().unwrap().buffer()).unwrap();

    let frame_info = boot_info.framebuffer.as_ref().unwrap().info();
    let framebuffer = boot_info.framebuffer.as_mut().unwrap();
    screen::init(framebuffer);
    for x in 0..frame_info.width {
        screenwriter().draw_pixel(x, frame_info.height-15, 0xff, 0, 0);
        screenwriter().draw_pixel(x, frame_info.height-10, 0, 0xff, 0);
        screenwriter().draw_pixel(x, frame_info.height-5, 0, 0, 0xff);
    }

    for r in boot_info.memory_regions.iter() {
        writeln!(serial(), "{:?} {:?} {:?} {}", r, r.start as *mut u8, r.end as *mut usize, r.end-r.start).unwrap();
    }

    let usable_region = boot_info.memory_regions.iter().filter(|x|x.kind == MemoryRegionKind::Usable).last().unwrap();
    writeln!(serial(), "{usable_region:?}").unwrap();

    let physical_offset = boot_info.physical_memory_offset.take().expect("Failed to find physical memory offset");
    let ptr = (physical_offset + usable_region.start) as *mut u8;
    writeln!(serial(), "Physical memory offset: {:X}; usable range: {:p}", physical_offset, ptr).unwrap();

    // print out values stored in specific memory address
    let vault = unsafe { slice::from_raw_parts_mut(ptr, 100) };
    vault[0] = 65;
    vault[1] = 66;
    writeln!(Writer, "{} {}", vault[0] as char, vault[1] as char).unwrap();

    //read CR3 for current page table
    let cr3 = Cr3::read().0.start_address().as_u64();
    writeln!(serial(), "CR3 read: {:#x}", cr3).unwrap();

    let cr3_page = unsafe { slice::from_raw_parts_mut((cr3 + physical_offset) as *mut usize, 6) };
    writeln!(serial(), "CR3 Page table virtual address {cr3_page:#p}").unwrap();

    allocator::init_heap((physical_offset + usable_region.start) as usize);

    let rsdp = boot_info.rsdp_addr.take();
    let mut mapper = frame_allocator::init(VirtAddr::new(physical_offset));
    let mut frame_allocator = BootInfoFrameAllocator::new(&boot_info.memory_regions);
    
    gdt::init();

    // Initialize pong game before starting the kernel
    pong::init_game();
    
    // print out values from heap allocation
    let x = Box::new(42);
    let y = Box::new(24);
    writeln!(Writer, "x + y = {}", *x + *y).unwrap();
    writeln!(Writer, "{x:#p} {:?}", *x).unwrap();
    writeln!(Writer, "{y:#p} {:?}", *y).unwrap();
    
    writeln!(serial(), "Starting kernel...").unwrap();

    let lapic_ptr = interrupts::init_apic(rsdp.expect("Failed to get RSDP address") as usize, physical_offset, &mut mapper, &mut frame_allocator);
    HandlerTable::new()
        .keyboard(key)
        .timer(tick)
        .startup(start)
        .start(lapic_ptr)
}

fn start() {
    writeln!(Writer, "Welcome to Pong OS!").unwrap();
}

fn tick() {
    // Update the game state on each timer tick
    pong::update_game();
}

fn key(key: DecodedKey) {
    // Debug output to see what keys are being detected
    writeln!(serial(), "Key detected: {:?}", key).unwrap();
    
    match key {
        DecodedKey::Unicode(character) => {
            match character {
                'w' => {
                    // Direct key state setting - no toggling
                    pong::set_key_w(true);
                    writeln!(serial(), "W key pressed").unwrap();
                },
                's' => {
                    pong::set_key_s(true);
                    writeln!(serial(), "S key pressed").unwrap();
                },
                ' ' => {
                    pong::start_game();
                    // Reset only left paddle key states
                    pong::set_key_w(false);
                    pong::set_key_s(false);
                    writeln!(serial(), "Space pressed - game started").unwrap();
                },
                'q' => {
                    // Release left paddle keys
                    pong::set_key_w(false);
                    pong::set_key_s(false);
                    writeln!(serial(), "Keys released with Q").unwrap();
                },
                _ => write!(Writer, "{}", character).unwrap(),
            }
        },
        DecodedKey::RawKey(key) => {
            writeln!(serial(), "Raw key: {:?}", key).unwrap();
            // Only handle W and S raw key codes
            match key {
                KeyCode::W => pong::set_key_w(true),
                KeyCode::S => pong::set_key_s(true),
                _ => write!(Writer, "{:?}", key).unwrap(),
            }
        },
    }
}
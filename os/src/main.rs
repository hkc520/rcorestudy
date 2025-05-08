//! The main module and entrypoint
//!
//! Various facilities of the kernels are implemented as submodules. The most
//! important ones are:
//!
//! - [`trap`]: Handles all cases of switching from userspace to the kernel
//! - [`task`]: Task management
//! - [`syscall`]: System call handling and implementation
//! - [`mm`]: Address map using SV39
//! - [`sync`]: Wrap a static data structure inside it so that we are able to access it without any `unsafe`.
//! - [`fs`]: Separate user from file system with some structures
//!
//! The operating system also starts in this module. Kernel code starts
//! executing from `entry.asm`, after which [`rust_main()`] is called to
//! initialize various pieces of functionality. (See its source code for
//! details.)
//!
//! We then call [`task::run_tasks()`] and for the first time go to
//! userspace.

#![no_std]
#![no_main]
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]
#![allow(dead_code)]
#![feature(trait_upcasting)]
#![feature(ascii_char)]
#![feature(negative_impls)]

use core::arch::{asm, global_asm};

#[macro_use]
extern crate log;

extern crate alloc;

#[macro_use]
extern crate bitflags;

mod boards;

#[macro_use]
mod console;
pub mod block;
pub mod config;
pub mod drivers;
// pub mod fs;
pub mod fs;
pub mod lang_items;
pub mod logging;
pub mod mm;
pub mod sbi;
pub mod sync;
pub mod syscall;
pub mod task;
pub mod timer;
pub mod trap;
pub mod utils;

use boards::{shutdown, CLOCK_FREQ};
use config::{KERNEL_SPACE_OFFSET, MEMORY_END};
use riscv::register::satp;
use sbi::console_putchar;
use timer::{get_time, get_time_ms, sleep_ms};
use utils::platform_info::{init_dtb, machine_info, machine_info_from_dtb};

#[cfg(feature = "qemu")]
global_asm!(include_str!("entry.S"));

#[cfg(feature = "visionfive2")]
global_asm!(include_str!("entry_visionfive2.S"));

global_asm!(include_str!("link_initproc.S"));

fn clear_bss() {
    extern "C" {
        fn sbss();
        fn ebss();
    }
    unsafe {
        core::slice::from_raw_parts_mut(sbss as usize as *mut u8, ebss as usize - sbss as usize)
            .fill(0);
    }
}

#[no_mangle]
fn show_logo() {
    println!(
        r#"
    mmmm     mmmm                                                          .d88888b.   .d8888b.
    mmmmm   mmmmm                         kk                              d88P" "Y88b d88P  Y88b
   mm  mm   mm  mm                        kk                              888     888 Y88b.
   mm  mm   mm  mm     ooooo     nnnnnn   kk  kk     eeeee    yyy    yy   888     888  "Y888b.
  mmm   mm mmm  mmm   oo   oo   nn    nn  kk kk    eee   eee   yy    yy   888     888     "Y88b.
  mmm    mmmm   mmm  oo     oo  nn    nn  kkkk     eeeeeeeee   yy    yy   888     888       "888
 mmm      mmm    mmm  oo   oo   nn    nn  kk kkk   eee         yyy  yyy   Y88b. .d88P Y88b  d88P
 mmm      mmm    mmm   ooooo    nn    nn  kk   kkk  eeeeeee     yyyyyyy    "Y88888P"   "Y8888P" 
                                                                    yyy
                                                                    yyy  
                                                               yyyyyyy

"#
    );
}

#[no_mangle]
pub fn fake_main() {
    unsafe {
        asm!("add sp, sp, {}", in(reg) KERNEL_SPACE_OFFSET << 12);
        asm!("la t0, rust_main");
        asm!("add t0, t0, {}", in(reg) KERNEL_SPACE_OFFSET << 12);
        asm!("jalr zero, 0(t0)");
    }
}

#[no_mangle]
/// the rust entry-point of os
pub fn rust_main() -> ! {
    #[cfg(feature = "visionfive2")]
    // sleep 5 seconds to wait for the test program to connect
    sleep_ms(5000);
    println!("Hello, world!\n");
    show_logo();
    clear_bss();
    println!("[kernel] Hello, world!");
    logging::init();
    info!("logging init done");
    let satp = satp::read();
    info!(" satp: {:#x}", satp.bits());
    #[cfg(feature = "visionfive2")]
    init_dtb(None);
    #[cfg(feature = "qemu")]
    init_dtb(None);
    let machine_info = machine_info();
    #[cfg(feature = "visionfive2")]
    mm::init(machine_info.memory.end);
    #[cfg(feature = "qemu")]
    mm::init(MEMORY_END);
    info!("mm init done");
    mm::remap_test();
    info!("mm remap test done");
    trap::init();
    info!("trap init done");
    trap::enable_timer_interrupt();
    info!("timer interrupt enabled");
    timer::set_next_trigger();
    info!("timer set next trigger done");
    // for file in ALL_TASKS.iter() {
    //     task::add_file(file);
    //     task::run_tasks();
    // }
    info!("init file system");
    fs::init();
    info!("adding initproc");
    task::add_initproc();
    info!("running tasks");
    task::run_tasks();
    println!("[kernel] All tasks finished successfully!");
    println!("[kernel] ChaOS is shutting down...");
    shutdown();
}

unsafe fn vf2_debug_print(s: &str) {
    const SBI_CONSOLE_PUTCHAR: usize = 1;

    for &c in s.as_bytes() {
        asm!(
            "li a7, 1",
            "li a6, 0",
            "ecall",
            in("a0") c as usize,
            out("a7") _,
            out("a6") _,
        );
    }
}

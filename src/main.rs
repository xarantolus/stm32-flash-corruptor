#![no_std]
#![no_main]

use cortex_m_rt::{entry, exception};
use stm32l4::stm32l4r5;

// Which address should be corrupted, with an allowed range
const APPROXIMATE_ADDRESS_TO_CORRUPT: usize = 0x2000;
const CORRUPT_RANGE: usize = 0x8;
static_assertions::const_assert!(CORRUPT_RANGE > 0);

// On the first page, this tool itself lies. Don't let it erase itself!
// In dual bank mode, the first page is 4096 bytes, so we can't corrupt the first page.
// If you are in single-bank mode, don't go below 8192
static_assertions::const_assert!(APPROXIMATE_ADDRESS_TO_CORRUPT >= 8192);

mod flash;
mod hw;

use flash::*;
use hw::*;

#[panic_handler]
fn panic_handler(_info: &core::panic::PanicInfo) -> ! {
    set_red_led(true);

    let peripherals = unsafe { stm32l4r5::Peripherals::steal() };

    // Clear backup register zero - allows manual reset
    peripherals.RTC.bkpr[0].write(|w| unsafe { w.bits(0) });

    loop {
        watchdog_feed_min(&peripherals.IWDG);
    }
}

macro_rules! bad_thing_happened {
    () => {{
        // Turns on the green LED
        let peripherals = unsafe { stm32l4r5::Peripherals::steal() };

        let reg_content = peripherals.FLASH.eccr.read();
        let is_flash_nmi: bool = {
            let flash = Flash::new(peripherals.FLASH);
            if flash.is_dualbank() {
                // In dual-bank mode, Bit 29 (ECCD2) is reserved, so only look at bit 31 (ECCD)
                reg_content.eccd().bit_is_set()
            } else {
                /// Bit 31 and Bit 29 - either lower or upper 64 bits of 128 bit value
                const ECCD_ECCD2_MASK: u32 = 0xa0000000;
                reg_content.bits() & ECCD_ECCD2_MASK != 0
            }
        };

        let dead_addr = reg_content.addr_ecc().bits();

        // If this is an ECC error in the area we wanted, turn on the green LED
        if is_flash_nmi {
            if dead_addr >= APPROXIMATE_ADDRESS_TO_CORRUPT as u32
                && dead_addr < (APPROXIMATE_ADDRESS_TO_CORRUPT + CORRUPT_RANGE) as u32
            {
                // We're done!
                set_green_led(true);

                loop {
                    watchdog_feed(&peripherals.IWDG);
                }
            } else {
                set_red_led(true);
            }
        } else {
            set_red_led(true);
            set_blue_led(true);
        }

        loop {}
    }};
}

// Could reduce binary size by kind of just pointing these to the same function...
// on the other hand, I don't care
#[exception]
unsafe fn HardFault(_: &cortex_m_rt::ExceptionFrame) -> ! {
    bad_thing_happened!()
}

#[exception]
unsafe fn NonMaskableInt() -> ! {
    bad_thing_happened!()
}

#[exception]
unsafe fn DefaultHandler(_irqn: i16) -> ! {
    bad_thing_happened!()
}

const STATE_BEFORE_WRITE: u32 = 1;
const STATE_AFTER_WRITE: u32 = 2;

const MAGIC_VALUE: u32 = 0x99999999;

#[entry]
fn main() -> ! {
    let peripherals = unsafe { stm32l4r5::Peripherals::steal() };
    // For backup register access
    hw::enable_rtc(&peripherals.RCC, &peripherals.RTC, &peripherals.PWR);

    // Basically detect the first boot and set the top/bottom of the range
    let magic_val = peripherals.RTC.bkpr[0].read().bits();
    if magic_val != MAGIC_VALUE {
        // Note that we're no longer in the first boot
        peripherals.RTC.bkpr[0].write(|w| unsafe { w.bits(MAGIC_VALUE) });

        // Register 1 and 2 store the bottom and top of the range
        peripherals.RTC.bkpr[1].write(|w| unsafe { w.bits(100) });
        // In my tests, usually a value of just below ~400k is fine, but it's a bit random
        peripherals.RTC.bkpr[2].write(|w| unsafe { w.bits(1_000_000) });
        peripherals.RTC.bkpr[3].write(|w| unsafe { w.bits(0) });
    }

    // This is a reset counter, which is interesting when debugging
    peripherals.RTC.bkpr[4].modify(|r, w| unsafe { w.bits(r.bits() + 1) });

    let mut bottom = peripherals.RTC.bkpr[1].read().bits();
    let mut top = peripherals.RTC.bkpr[2].read().bits();
    let mut middle = (bottom + top) / 2;

    // If we are very close, we have likely missed the exact time and need to try again
    let very_similar = top - bottom < 5;
    assert!(!very_similar);

    let state = peripherals.RTC.bkpr[3].read().bits();

    if state == STATE_BEFORE_WRITE {
        // Apparently we run too long before the reset, so we need to go down
        top = middle;
        peripherals.RTC.bkpr[2].write(|w| unsafe { w.bits(top) });
    } else if state == STATE_AFTER_WRITE {
        // Apparently reset too late, so go up a bit
        bottom = middle;
        peripherals.RTC.bkpr[1].write(|w| unsafe { w.bits(bottom) });
    }
    middle = (bottom + top) / 2;

    peripherals.RTC.bkpr[3].write(|w| unsafe { w.bits(STATE_BEFORE_WRITE) });

    // We basically do a binary search over multiple resets to find the right time to corrupt,
    // but with the similarity check we also jump around a bit to avoid getting stuck

    set_green_led(false);
    set_red_led(false);
    set_blue_led(false);

    // First of all, read all of that data to see if we get an interrupt
    // If yes, we are already in a corrupted state - nice!
    for i in 0..CORRUPT_RANGE {
        let addr = (APPROXIMATE_ADDRESS_TO_CORRUPT as usize) + i;

        let data = unsafe { core::ptr::read_volatile(addr as *const u8) };

        core::hint::black_box(data);
    }

    // If we reach this, there is no corruption in the aimed area
    let mut flash = Flash::new(peripherals.FLASH);
    let page_number = flash.address_to_page_number(APPROXIMATE_ADDRESS_TO_CORRUPT as u32);

    // We use the watchdog to time the corruption (I think? lol)
    activate_watchdog(&peripherals.IWDG).unwrap();

    // First of all, we erase the page, as otherwise we can't write to it
    let mut flash_unlocked = flash.unlock().unwrap();
    flash_unlocked.erase_page(page_number).unwrap();

    // After this, we have 0.125ms until we have to be within a write
    watchdog_feed_min(&peripherals.IWDG);

    // This gets us towards the time window...
    // Also it probably isn't exactly cycles, but close enough
    for _ in 0..middle {
        core::hint::black_box(0);
    }

    // ...and this is the write that actually corrupts the flash
    flash_unlocked
        .write_dwords(
            &peripherals.SCB_ACTRL,
            APPROXIMATE_ADDRESS_TO_CORRUPT as *mut usize,
            &[0u64; CORRUPT_RANGE / core::mem::size_of::<u64>() + 1],
        )
        .unwrap();

    peripherals.RTC.bkpr[3].write(|w| unsafe { w.bits(STATE_AFTER_WRITE) });
    set_blue_led(true);

    loop {
        // Wait for the watchdog to reset us
    }
}

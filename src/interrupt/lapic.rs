// LAPIC
// Reference:
//  - https://wiki.osdev.org/APIC#Local_APIC_configuration
//  - https://github.com/pdoane/osdev/blob/master/intr/local_apic.c
//  - https://github.com/mit-pdos/xv6-public/blob/master/lapic.c

extern crate raw_cpuid;
use core::ptr;
use x86_64::registers::model_specific::Msr;
use super::InterruptIndex;

static mut lapic: u32 = 0;

const LAPIC_ID: u32 = 0x0020;
const LAPIC_VER: u32 = 0x0030;
const LAPIC_TPR: u32 = 0x0080;
const LAPIC_EOI: u32 = 0x00b0;
const LAPIC_SVR: u32 = 0x00f0;
const LAPIC_ESR: u32 = 0x0280;
const LAPIC_TIMER: u32 = 0x0320;
const LAPIC_PCINT: u32 = 0x0340;
const LAPIC_LINT0: u32 = 0x0350;
const LAPIC_LINT1: u32 = 0x0360;
const LAPIC_ERROR: u32 = 0x0370;
const LAPIC_TICR: u32 = 0x0380;
const LAPIC_TDCR: u32 = 0x03e0;

const LAPIC_SVR_ENABLE: u32 = 0x0100;
const LAPIC_TDCR_X1: u32 = 0x0000000b;
const LAPIC_TIMER_PERIODIC: u32 = 0x00020000;
const LAPIC_MASKED: u32 = 0x00010000;

const IRQ_OFFSET: u32 = super::IRQ_OFFSET as u32;
const IRQ_SPURIOUS: u32 = 31;

#[no_mangle]
unsafe fn lapicr(offset: u32) -> u32 {
    ptr::read_volatile((lapic + offset) as *const u32)
}

#[no_mangle]
unsafe fn lapicw(offset: u32, value: u32) {
    ptr::write_volatile((lapic + offset) as *mut u32, value);
    ptr::read_volatile((lapic + LAPIC_ID) as *const u32);
}

fn probe_apic() {
    unsafe {
        let msr27: u32 = Msr::new(27).read() as u32;
        lapic = msr27 & 0xffff0000;
    }
}

fn init_lapic() {
    unsafe {
        // Enable LAPIC
        lapicw(LAPIC_SVR, LAPIC_SVR_ENABLE | (IRQ_OFFSET + IRQ_SPURIOUS));

        // Timer interrupt
        lapicw(LAPIC_TDCR, LAPIC_TDCR_X1);
        lapicw(LAPIC_TIMER, LAPIC_TIMER_PERIODIC | InterruptIndex::Timer.as_u32());
        lapicw(LAPIC_TICR, 10000000);

        // Mask logical interrupt lines
        lapicw(LAPIC_LINT0, LAPIC_MASKED);
        lapicw(LAPIC_LINT1, LAPIC_MASKED);

        // Mask performance counter overflow interrupts
        if ((lapicr(LAPIC_VER) >> 16) & 0xff) >= 4 {
            lapicw(LAPIC_PCINT, LAPIC_MASKED);
        }

        // Remap error
        lapicw(LAPIC_ERROR, InterruptIndex::ApicError.as_u32());

        // Clear error status register
        lapicw(LAPIC_ESR, 0);
        lapicw(LAPIC_ESR, 0);

        // Ack any outstanding interrupts
        lapicw(LAPIC_EOI, 0);

        // Enable interrupts on APIC
        lapicw(LAPIC_TPR, 0);
    }
}

pub fn end_of_interrupt() {
    unsafe {
        lapicw(LAPIC_EOI, 0);
    }
}

pub fn init() {
    probe_apic();
    init_lapic();
}
use core::cell::Cell;

use arduino_hal::pac::TC0;
use avr_device::{
    interrupt,
    interrupt::{enable, free, Mutex},
};

const PRESCALER: u32 = 256;
const TIMER_COUNTS: u32 = 250;
const MILLIS_INCREMENT: u32 = PRESCALER * TIMER_COUNTS / 16_000;
static MILLIS_COUNTER: Mutex<Cell<u32>> = Mutex::new(Cell::new(0));

pub fn init(tc0: &TC0) {
    tc0.tccr0a.write(|w| w.wgm0().ctc());
    tc0.ocr0a
        .write(|w| w.bits(u8::try_from(TIMER_COUNTS).unwrap()));
    tc0.tccr0b.write(|w| match PRESCALER {
        8 => w.cs0().prescale_8(),
        64 => w.cs0().prescale_64(),
        256 => w.cs0().prescale_256(),
        1024 => w.cs0().prescale_1024(),
        _ => panic!(),
    });
    tc0.timsk0.write(|w| w.ocie0a().set_bit());

    free(|cs| {
        MILLIS_COUNTER.borrow(cs).set(0);
    });

    unsafe { enable() };
}

#[interrupt(atmega328p)]
fn TIMER0_COMPA() {
    free(|cs| {
        let counter_cell = MILLIS_COUNTER.borrow(cs);
        let counter = counter_cell.get();
        counter_cell.set(counter + MILLIS_INCREMENT);
    });
}

pub fn millis() -> u32 {
    free(|cs| MILLIS_COUNTER.borrow(cs).get())
}

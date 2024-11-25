#![feature(abi_avr_interrupt, array_chunks)]
#![no_std]
#![no_main]

use crate::game::Game;
use arduino_hal::Peripherals;
use hexapawn::Hexapawn;
use millis::init;
use panic_halt as _;
use ttt::TicTacToe;

mod buttons;
mod display;
mod hexapawn;
mod millis;
mod ttt;

mod consts {
    pub const SIZE: usize = 3;
}
mod board {
    use crate::consts::SIZE;
    pub type Board<C> = [[C; SIZE]; SIZE];
}

mod player {
    #[derive(Copy, Clone, PartialEq, Eq)]
    pub enum Player {
        Red,
        Blue,
    }

    impl Player {
        pub fn flip(&mut self) {
            *self = match self {
                Self::Red => Self::Blue,
                Self::Blue => Self::Red,
            }
        }
    }

    macro_rules! choose {
        ($peripherals:expr, $pins:expr) => {{
            use arduino_hal::adc::AdcSettings;
            use avr_hal_generic::{adc::Adc, clock::MHz16};
            let mut adc: Adc<_, _, MHz16> = Adc::new($peripherals.ADC, AdcSettings::default());
            if $pins.a5.into_analog_input(&mut adc).analog_read(&mut adc) & 1 == 0 {
                Player::Red
            } else {
                Player::Blue
            }
        }};
    }

    pub(crate) use choose;
}

mod game {
    pub trait Game {
        fn play() -> !;
    }
}

mod position {
    use crate::consts::SIZE;

    #[derive(Clone, Copy, PartialEq, Eq)]
    pub struct Position {
        pub x: usize,
        pub y: usize,
    }

    pub const fn pos(x: usize, y: usize) -> Position {
        Position { x, y }
    }

    impl Position {
        pub fn offset_y(self, delta: isize) -> Option<Self> {
            let y = self.y.checked_add_signed(delta)?;
            if y < SIZE {
                Some(pos(self.x, y))
            } else {
                None
            }
        }

        pub fn offset_x(self, delta: isize) -> Option<Self> {
            let x = self.x.checked_add_signed(delta)?;
            if x < SIZE {
                Some(pos(x, self.y))
            } else {
                None
            }
        }
    }
}

#[arduino_hal::entry]
fn main() -> ! {
    unsafe extern "C" {
        static mut DEVICE_PERIPHERALS: bool;
    }
    unsafe {
        init(&Peripherals::steal().TC0);
        DEVICE_PERIPHERALS = false;
    }

    match 1 {
        0 => TicTacToe::play(),
        1 => Hexapawn::play(),
        _ => unreachable!(),
    }
}

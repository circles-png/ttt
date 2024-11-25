use crate::millis::millis;
use core::{array::from_fn, convert::Infallible};

use arduino_hal::{
    hal::port::Dynamic,
    port::mode::{OpenDrain, Output},
    prelude::_unwrap_infallible_UnwrapInfallible,
};
use avr_hal_generic::{hal_v0::digital::v2::OutputPin, port::Pin};

use crate::{board::Board, consts::SIZE, player::Player};

#[derive(Clone, Copy, Default)]
pub struct DisplayPixel {
    red: bool,
    blue: bool,
}

impl DisplayPixel {
    pub const fn filter(&mut self, bool: bool) -> &mut Self {
        self.red &= bool;
        self.blue &= bool;
        self
    }

    pub const fn and(self, bool: bool) -> Self {
        Self {
            red: self.red && bool,
            blue: self.blue && bool,
        }
    }

    pub const fn red() -> Self {
        Self {
            red: true,
            blue: false,
        }
    }

    pub const fn blue() -> Self {
        Self {
            red: false,
            blue: true,
        }
    }

    pub const fn red_if(cond: bool) -> Self {
        Self {
            red: cond,
            blue: !cond,
        }
    }

    pub fn or_player(&mut self, player: Player) -> &mut Self {
        (*match player {
            Player::Red => &mut self.red,
            Player::Blue => &mut self.blue,
        }) = true;
        self
    }

    pub fn write_to(
        self,
        red_pin: &mut Pin<Output, Dynamic>,
        blue_pin: &mut Pin<Output, Dynamic>,
    ) -> Result<(), Infallible> {
        red_pin.set_state(self.red.into())?;
        blue_pin.set_state(self.blue.into())?;
        Ok(())
    }
}

impl From<Player> for DisplayPixel {
    fn from(player: Player) -> Self {
        match player {
            Player::Red => Self::red(),
            Player::Blue => Self::blue(),
        }
    }
}

pub struct Display {
    buffer: Board<DisplayPixel>,
    blue_columns: [Pin<Output, Dynamic>; SIZE],
    red_columns: [Pin<Output, Dynamic>; SIZE],
    led_rows: [Pin<OpenDrain, Dynamic>; SIZE],
}

impl Display {
    pub const fn new(
        buffer: Board<DisplayPixel>,
        blue_columns: [Pin<Output, Dynamic>; SIZE],
        red_columns: [Pin<Output, Dynamic>; SIZE],
        led_rows: [Pin<OpenDrain, Dynamic>; SIZE],
    ) -> Self {
        Self {
            buffer,
            blue_columns,
            red_columns,
            led_rows,
        }
    }

    pub fn write(&mut self, buffer: Board<DisplayPixel>) {
        self.buffer = buffer;
    }

    pub fn show(&mut self) {
        let y = millis() as usize % SIZE;
        for column in [self.red_columns.each_mut(), self.blue_columns.each_mut()].as_flattened_mut()
        {
            column.set_low();
        }
        for (row, active) in self
            .led_rows
            .iter_mut()
            .zip(from_fn::<_, SIZE, _>(|index| index == y))
        {
            row.set_state((!active).into()).unwrap_infallible();
        }
        let row = &self.buffer[y];
        for (x, pixel) in row.iter().enumerate() {
            pixel
                .write_to(&mut self.red_columns[x], &mut self.blue_columns[x])
                .unwrap_infallible();
        }
    }
}

macro_rules! display {
    ($pins:expr, $initial: expr) => {{
        use crate::{display::Display, pin_triple};
        Display::new(
            $initial,
            pin_triple!($pins, into_output, d2, d4, d6),
            pin_triple!($pins, into_output, d3, d5, d7),
            pin_triple!($pins, into_opendrain, d8, d9, d10),
        )
    }};
}

pub(crate) use display;

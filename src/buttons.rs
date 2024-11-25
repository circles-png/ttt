use arduino_hal::{
    hal::port::Dynamic,
    port::mode::{Input, OpenDrain, PullUp},
};
use avr_hal_generic::{hal_v0::digital::v2::OutputPin, port::Pin};
use core::{array::from_fn, convert::identity};
use itertools::Itertools;

use crate::{board::Board, consts::SIZE, position::{pos, Position}};

pub struct Buttons {
    rows: [Pin<OpenDrain, Dynamic>; SIZE],
    columns: [Pin<Input<PullUp>, Dynamic>; SIZE],
}

pub struct ButtonScan(Board<bool>);

impl ButtonScan {
   pub  fn exactly_one(&self) -> Option<Position> {
        self.0
            .into_iter()
            .enumerate()
            .filter_map(|(y, row)| {
                row.into_iter()
                    .enumerate()
                    .filter_map(|(x, pressed)| pressed.then_some(x))
                    .exactly_one()
                    .ok()
                    .map(|x| pos(x, y))
            })
            .exactly_one()
            .ok()
    }
}

impl Buttons {
    pub fn scan(&mut self) -> ButtonScan {
        ButtonScan(from_fn(identity).map(|y| {
            for (index, other) in self.rows.iter_mut().enumerate() {
                other.set_state((index != y).into()).unwrap();
            }
            self.columns.each_ref().map(<Pin<Input<_>, _>>::is_low)
        }))
    }

    pub const fn new(
        rows: [Pin<OpenDrain, Dynamic>; SIZE],
        columns: [Pin<Input<PullUp>, Dynamic>; SIZE],
    ) -> Self {
        Self { rows, columns }
    }
}

#[macro_export]
macro_rules! pin_triple {
    ($pins:expr, $mode:ident, $($pin:ident),+) => {
        [$($pins.$pin.$mode().downgrade()),+]
    };
}

macro_rules! buttons {
    ($pins:expr) => {{
        use crate::{buttons::Buttons, pin_triple};
        Buttons::new(
            pin_triple!($pins, into_opendrain, a0, a1, a2),
            pin_triple!($pins, into_pull_up_input, d11, d12, a3),
        )
    }};
}

pub(crate) use buttons;

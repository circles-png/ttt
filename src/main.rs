#![feature(abi_avr_interrupt, array_chunks)]
#![no_std]
#![no_main]

use core::{
    array::from_fn,
    convert::{identity, Into},
};

use arduino_hal::{
    adc::AdcSettings,
    hal::port::Dynamic,
    pins,
    port::mode::{Input, OpenDrain, Output, PullUp},
    Adc, Peripherals,
};
use avr_hal_generic::{hal_v0::digital::v2::OutputPin, port::Pin};
use itertools::Itertools;
use millis::{init, millis};
use panic_halt as _;
mod millis;

struct Game {
    board: Board,
    current_player: Player,
    display: Board<DisplayPixel>,
    state: GameState,
    blue_columns: [Pin<Output, Dynamic>; 3],
    red_columns: [Pin<Output, Dynamic>; 3],
    led_rows: [Pin<OpenDrain, Dynamic>; 3],
    button_columns: [Pin<Input<PullUp>, Dynamic>; 3],
    button_rows: [Pin<OpenDrain, Dynamic>; 3],
}

impl Game {
    fn new() -> Self {
        let peripherals = Peripherals::take().unwrap();
        let pins = pins!(peripherals);
        init(&peripherals.TC0);
        macro_rules! p {
            ($mode:ident, $($pins:ident),+) => {
                [$(pins.$pins.$mode().downgrade()),+]
            };
        }
        let current_player = {
            let mut adc = Adc::new(peripherals.ADC, AdcSettings::default());
            if pins.a5.into_analog_input(&mut adc).analog_read(&mut adc) & 1 == 0 {
                Player::Red
            } else {
                Player::Blue
            }
        };
        Self {
            board: [[None; 3]; 3],
            current_player,
            display: [[DisplayPixel::from(current_player); 3]; 3],
            state: GameState::default(),
            blue_columns: p!(into_output, d2, d4, d6),
            red_columns: p!(into_output, d3, d5, d7),
            led_rows: p!(into_opendrain, d8, d9, d10),
            button_columns: p!(into_pull_up_input, d11, d12, a3),
            button_rows: p!(into_opendrain, a0, a1, a2),
        }
    }

    fn run(mut self) {
        loop {
            match self.state {
                GameState::PresentCurrentPlayer => {
                    if millis() > 1000 {
                        self.state = GameState::WaitForMove;
                    }
                }
                GameState::WaitForMove => {
                    if let Ok((x, y)) = self
                        .get_buttons()
                        .into_iter()
                        .enumerate()
                        .filter_map(|(y, row)| {
                            row.into_iter()
                                .enumerate()
                                .filter_map(|(x, pressed)| pressed.then_some(x))
                                .exactly_one()
                                .ok()
                                .map(|x| (x, y))
                        })
                        .exactly_one()
                    {
                        if self.board[y][x].is_none() {
                            self.board[y][x] = Some(self.current_player);
                            self.current_player.flip();
                        };
                    }
                    self.display = self.board.map(|row| row.map(Into::into));
                    if let Some(winner) = self.winner() {
                        self.state = GameState::DisplayWinner(winner);
                    }
                    if self.board.as_flattened().iter().all(Option::is_some) {
                        self.state = GameState::DisplayTie;
                    }
                }
                GameState::DisplayWinner(winner) => {
                    let mut pixels = from_fn::<_, { 3 * 3 }, _>(|index| {
                        DisplayPixel::from(winner)
                            .and(index % 2 == usize::from(millis() % 1000 < 500))
                    })
                    .into_iter();
                    self.display = from_fn(|_| from_fn(|_| pixels.next().unwrap()));
                }
                GameState::DisplayTie => {
                    let mut pixels = from_fn::<_, { 3 * 3 }, _>(|index| DisplayPixel {
                        red: index % 2 == usize::from(millis() % 1000 < 500),
                        blue: index % 2 != usize::from(millis() % 1000 < 500),
                    })
                    .into_iter();
                    self.display = from_fn(|_| from_fn(|_| pixels.next().unwrap()));
                }
            }
            self.show_display();
        }
    }

    fn get_buttons(&mut self) -> Board<bool> {
        from_fn(identity).map(|y| {
            for (index, other) in self.button_rows.iter_mut().enumerate() {
                other.set_state((index != y).into()).unwrap();
            }
            self.button_columns
                .each_ref()
                .map(<Pin<Input<_>, _>>::is_low)
        })
    }

    fn show_display(&mut self) {
        let y = (millis() % 3) as usize;
        for column in [self.red_columns.each_mut(), self.blue_columns.each_mut()].as_flattened_mut()
        {
            column.set_state(false.into()).unwrap();
        }
        for (row, active) in self
            .led_rows
            .iter_mut()
            .zip(from_fn::<_, 3, _>(|index| index == y))
        {
            row.set_state((!active).into()).unwrap();
        }
        let row = &self.display[y];
        for (x, pixel) in row.iter().enumerate() {
            self.red_columns[x].set_state(pixel.red.into()).unwrap();
            self.blue_columns[x].set_state(pixel.blue.into()).unwrap();
        }
    }

    fn winner(&self) -> Option<Player> {
        for row in &self.board {
            if let Ok(&Some(player)) = row.iter().all_equal_value() {
                return Some(player);
            }
        }
        for x in 0..3 {
            if let Ok(Some(player)) = self.board.iter().map(|row| row[x]).all_equal_value() {
                return Some(player);
            }
        }
        if let Ok(Some(player)) = from_fn::<_, 3, _>(|i| self.board[i][i])
            .into_iter()
            .all_equal_value()
        {
            return Some(player);
        }
        if let Ok(Some(player)) = from_fn::<_, 3, _>(|i| self.board[i][2 - i])
            .into_iter()
            .all_equal_value()
        {
            return Some(player);
        }
        None
    }
}

#[derive(Copy, Clone, PartialEq)]
enum Player {
    Red,
    Blue,
}

impl Player {
    fn flip(&mut self) {
        *self = match self {
            Self::Red => Self::Blue,
            Self::Blue => Self::Red,
        }
    }
}

#[derive(Default, Clone, Copy)]
enum GameState {
    #[default]
    PresentCurrentPlayer,
    WaitForMove,
    DisplayWinner(Player),
    DisplayTie,
}

type Cell = Option<Player>;
type Board<C = Cell> = [[C; 3]; 3];

#[derive(Clone, Copy, Default)]
struct DisplayPixel {
    red: bool,
    blue: bool,
}

impl DisplayPixel {
    const fn and(self, bool: bool) -> Self {
        Self {
            red: self.red && bool,
            blue: self.blue && bool,
        }
    }
}

impl From<Player> for DisplayPixel {
    fn from(player: Player) -> Self {
        match player {
            Player::Red => Self {
                red: true,
                blue: false,
            },
            Player::Blue => Self {
                red: false,
                blue: true,
            },
        }
    }
}

impl From<Cell> for DisplayPixel {
    fn from(cell: Cell) -> Self {
        cell.map(Into::into).unwrap_or_default()
    }
}

#[arduino_hal::entry]
fn main() -> ! {
    loop {
        Game::new().run();
    }
}

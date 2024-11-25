use core::{array::from_fn, convert::Into};

use arduino_hal::{pins, Peripherals};
use itertools::Itertools;
use millis::millis;

use crate::{
    board::Board,
    buttons::{buttons, Buttons},
    consts::SIZE,
    display::{display, Display, DisplayPixel},
    game::Game,
    millis,
    player::{choose, Player},
    position::Position,
};

pub struct TicTacToe {
    board: Board<Cell>,
    current_player: Player,
    display: Display,
    state: GameState,
    buttons: Buttons,
}

impl TicTacToe {
    pub fn new() -> Self {
        let peripherals = Peripherals::take().unwrap();
        let pins = pins!(peripherals);
        let current_player = choose!(peripherals, pins);
        Self {
            board: [[None; SIZE]; SIZE],
            current_player,
            display: display!(pins, [[DisplayPixel::from(current_player); SIZE]; SIZE]),
            state: GameState::default(),
            buttons: buttons!(pins),
        }
    }

    pub fn run(mut self) -> ! {
        loop {
            match self.state {
                GameState::PresentCurrentPlayer => {
                    if millis() > 1000 {
                        self.state = GameState::WaitForMove;
                    }
                }
                GameState::WaitForMove => {
                    if let Some(Position { x, y }) = self.buttons.scan().exactly_one() {
                        if self.board[y][x].is_none() {
                            self.board[y][x] = Some(self.current_player);
                            self.current_player.flip();
                        };
                    }
                    self.display
                        .write(self.board.map(|row| row.map(Into::into)));
                    if let Some(winner) = self.winner() {
                        self.state = GameState::DisplayWinner(winner);
                    }
                    if self.board.as_flattened().iter().all(Option::is_some) {
                        self.state = GameState::DisplayTie;
                    }
                }
                GameState::DisplayWinner(winner) => {
                    let mut pixels = from_fn::<_, { SIZE * SIZE }, _>(|index| {
                        DisplayPixel::from(winner)
                            .and(index % 2 == usize::from(millis() % 1000 < 500))
                    })
                    .into_iter();
                    self.display
                        .write(from_fn(|_| from_fn(|_| pixels.next().unwrap())));
                }
                GameState::DisplayTie => {
                    let mut pixels = from_fn::<_, { SIZE * SIZE }, _>(|index| {
                        DisplayPixel::red_if(index % 2 == usize::from(millis() % 1000 < 500))
                    })
                    .into_iter();
                    self.display
                        .write(from_fn(|_| from_fn(|_| pixels.next().unwrap())));
                }
            }
            self.display.show();
        }
    }

    fn winner(&self) -> Option<Player> {
        for row in &self.board {
            if let Ok(&Some(player)) = row.iter().all_equal_value() {
                return Some(player);
            }
        }
        for x in 0..SIZE {
            if let Ok(Some(player)) = self.board.iter().map(|row| row[x]).all_equal_value() {
                return Some(player);
            }
        }
        if let Ok(Some(player)) = from_fn::<_, SIZE, _>(|i| self.board[i][i])
            .into_iter()
            .all_equal_value()
        {
            return Some(player);
        }
        if let Ok(Some(player)) = from_fn::<_, SIZE, _>(|i| self.board[i][2 - i])
            .into_iter()
            .all_equal_value()
        {
            return Some(player);
        }
        None
    }
}

impl Game for TicTacToe {
    fn play() -> ! {
        Self::new().run();
    }
}

impl Default for TicTacToe {
    fn default() -> Self {
        Self::new()
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

impl From<Cell> for DisplayPixel {
    fn from(cell: Cell) -> Self {
        cell.map(Into::into).unwrap_or_default()
    }
}

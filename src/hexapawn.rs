use core::array::{from_fn, IntoIter};
use core::iter::Flatten;

use crate::board::Board;
use crate::position::pos;
use crate::{buttons::Buttons, display::Display, millis::millis, position::Position};

use arduino_hal::{pins, Peripherals};
use itertools::Itertools;

use crate::{
    buttons::buttons,
    consts::SIZE,
    display::{display, DisplayPixel},
    game::Game,
    player::{choose, Player},
};

pub struct Hexapawn {
    pawns: Pawns,
    current_player: Player,
    state: GameState,
    display: Display,
    buttons: Buttons,
}

#[derive(Clone, Copy)]
struct Pawns([Option<Pawn>; 6]);

impl Pawns {
    fn into_display_buffer(self) -> Board<DisplayPixel> {
        let mut buffer = [[DisplayPixel::default(); SIZE]; SIZE];
        for pawn in self {
            buffer[pawn.position.y][pawn.position.x] = DisplayPixel::from(pawn.player);
        }
        buffer
    }

    fn valid_next_moves(self, pawn: Pawn) -> impl Iterator<Item = Position> {
        let forward = pawn.position.offset_y(match pawn.player {
            Player::Red => 1,
            Player::Blue => -1,
        });
        let sides = [-1, 1].map(|delta| forward.and_then(|forward| forward.offset_x(delta)));
        forward
            .filter(|forward| !self.into_iter().any(|other| other.position == *forward))
            .into_iter()
            .chain(sides.into_iter().flatten().filter(move |side| {
                self.into_iter()
                    .any(|other| other.player != pawn.player && other.position == *side)
            }))
    }
}

impl IntoIterator for Pawns {
    type Item = Pawn;
    type IntoIter = Flatten<IntoIter<Option<Pawn>, 6>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter().flatten()
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct Pawn {
    position: Position,
    player: Player,
}

impl Pawn {
    const fn new(position: Position, player: Player) -> Self {
        Self { position, player }
    }
}

#[derive(Clone, Copy, Default)]
enum GameState {
    #[default]
    PresentCurrentPlayer,
    WaitForPick,
    WaitForPlace(Pawn),
    DisplayWinner(Player),
}

impl Hexapawn {
    pub fn new() -> Self {
        let peripherals = Peripherals::take().unwrap();
        let pins = pins!(peripherals);

        let current_player = choose!(peripherals, pins);
        Self {
            pawns: Pawns([
                Some(Pawn::new(pos(0, 0), Player::Red)),
                Some(Pawn::new(pos(1, 0), Player::Red)),
                Some(Pawn::new(pos(2, 0), Player::Red)),
                Some(Pawn::new(pos(0, SIZE - 1), Player::Blue)),
                Some(Pawn::new(pos(1, SIZE - 1), Player::Blue)),
                Some(Pawn::new(pos(2, SIZE - 1), Player::Blue)),
            ]),
            current_player,
            state: GameState::default(),
            display: display!(pins, [[DisplayPixel::from(current_player); SIZE]; SIZE]),
            buttons: buttons!(pins),
        }
    }

    fn run(mut self) -> ! {
        loop {
            match self.state {
                GameState::PresentCurrentPlayer => {
                    if millis() > 1000 {
                        self.state = GameState::WaitForPick;
                    }
                }
                GameState::WaitForPick => {
                    self.display.write(self.pawns.into_display_buffer());
                    if let Some(Position { x, y }) = self.buttons.scan().exactly_one() {
                        if let Ok(pawn) = self
                            .pawns
                            .into_iter()
                            .filter(|pawn| *pawn == Pawn::new(pos(x, y), self.current_player))
                            .exactly_one()
                        {
                            self.state = GameState::WaitForPlace(pawn);
                        }
                    }
                    if let Some(winner) = self.winner() {
                        self.state = GameState::DisplayWinner(winner);
                    }
                }
                GameState::WaitForPlace(pawn) => {
                    let mut buffer = self.pawns.into_display_buffer();
                    buffer[pawn.position.y][pawn.position.x].filter(millis() % 1000 < 500);
                    for position in self.pawns.valid_next_moves(pawn) {
                        if millis() % 1000 > 500 {
                            buffer[position.y][position.x].or_player(pawn.player);
                        }
                    }
                    self.display.write(buffer);
                    if let Some(new) = self.buttons.scan().exactly_one() {
                        if let Ok(pawn) = self
                            .pawns
                            .into_iter()
                            .filter(|pawn| *pawn == Pawn::new(new, self.current_player))
                            .exactly_one()
                        {
                            self.state = GameState::WaitForPlace(pawn);
                        } else if self
                            .pawns
                            .valid_next_moves(pawn)
                            .any(|position| position == new)
                        {
                            for other in &mut self.pawns.0 {
                                if other.is_some_and(|other| other.position == new) {
                                    *other = None;
                                }
                            }

                            self.pawns
                                .0
                                .iter_mut()
                                .flatten()
                                .filter(|other| other.position == pawn.position)
                                .exactly_one()
                                .ok()
                                .unwrap()
                                .position = new;
                            self.current_player.flip();
                            self.state = GameState::WaitForPick;
                        }
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
            }
            self.display.show();
        }
    }

    fn winner(&self) -> Option<Player> {
        if let Ok(player) = self
            .pawns
            .into_iter()
            .map(|pawn| pawn.player)
            .all_equal_value()
        {
            return Some(player);
        }
        if self
            .pawns
            .into_iter()
            .filter(|pawn| pawn.player == self.current_player)
            .all(|pawn| self.pawns.valid_next_moves(pawn).next().is_none())
        {
            let mut player = self.current_player;
            player.flip();
            return Some(player);
        }
        if let Ok(pawn) = self
            .pawns
            .into_iter()
            .filter(|pawn| {
                (match pawn.player {
                    Player::Red => SIZE - 1,
                    Player::Blue => 0,
                }) == pawn.position.y
            })
            .exactly_one()
        {
            return Some(pawn.player);
        }
        None
    }
}

impl Game for Hexapawn {
    fn play() -> ! {
        Self::new().run();
    }
}

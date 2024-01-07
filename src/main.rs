#![no_main]
#![no_std]

mod game;
mod control;
mod display;
mod sound;

use cortex_m_rt::entry;
use microbit::Board;
use rtt_target::rtt_init_print;
use microbit::hal::{Rng, Timer};
use microbit::display::blocking::Display;
use microbit::hal::prelude::*;
use panic_rtt_target as _;

use crate::control::{get_turn, init_buttons};
use crate::game::{Game, GameStatus};


#[entry]
fn main() -> ! {
    rtt_init_print!();
    let mut board = Board::take().unwrap();
    let mut timer = Timer::new(board.TIMER0);
    let rng = Rng::new(board.RNG);
    let mut game = Game::new(rng);

    init_buttons(board.GPIOTE, board.buttons);

    let mut display = Display::new(board.display_pins);

    loop {
        loop {  // Game loop
            let image = game.game_matrix(8, 4, 2);
            display.show(&mut timer, image, game.step_len_ms());
            match game.status {
                GameStatus::Ongoing => game.step(get_turn(true)),
                _ => {
                    for _ in 0..3 {
                        display.clear();
                        timer.delay_ms(200u32);
                        display.show(&mut timer, image, 200);
                    }
                    display.clear();
                    display.show(&mut timer, game.score_matrix(9), 1000);
                    break
                }
            }
        }
        game.reset();
    }
}

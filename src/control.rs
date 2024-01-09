// https://github.com/nrf-rs/microbit/blob/main/examples/gpio-hal-printbuttons/src/main.rs

use core::cell::RefCell;
use cortex_m::interrupt::{free, Mutex};
use microbit::board::Buttons;
use microbit::hal::gpiote::Gpiote;
use microbit::pac::{self, GPIOTE, interrupt};

#[derive(Debug, Copy, Clone)]
pub enum Turn {
    Left,
    Right,
    None
}

static GPIO: Mutex<RefCell<Option<Gpiote>>> = Mutex::new(RefCell::new(None));
static TURN: Mutex<RefCell<Turn>> = Mutex::new(RefCell::new(Turn::None));

pub(crate) fn init_buttons(board_gpiote: GPIOTE, board_buttons: Buttons) {
    let gpiote = Gpiote::new(board_gpiote);

    let channel0 = gpiote.channel0();
    channel0
        .input_pin(&board_buttons.button_a.degrade())
        .hi_to_lo()
        .enable_interrupt();
    channel0.reset_events();

    let channel1 = gpiote.channel1();
    channel1
        .input_pin(&board_buttons.button_b.degrade())
        .hi_to_lo()
        .enable_interrupt();
    channel1.reset_events();

    free(move |cs| {
        /* Enable external GPIO interrupts */
        unsafe {
            pac::NVIC::unmask(pac::Interrupt::GPIOTE);
        }
        pac::NVIC::unpend(pac::Interrupt::GPIOTE);
        *GPIO.borrow(cs).borrow_mut() = Some(gpiote);
    });

}

pub fn get_turn(reset: bool) -> Turn {
    free(|cs| {
        let turn = *TURN.borrow(cs).borrow();
        if reset {
            *TURN.borrow(cs).borrow_mut() = Turn::None
        }
        turn
    })
}

#[interrupt]
fn GPIOTE() {
    // Enter a critical section here to satisfy the Mutex.
    free(|cs| {
        if let Some(gpiote) = GPIO.borrow(cs).borrow().as_ref() {
            let a_pressed = gpiote.channel0().is_event_triggered();
            let b_pressed = gpiote.channel1().is_event_triggered();

            let turn = match (a_pressed, b_pressed) {
                (false, false) => Turn::None,
                (true, false) => Turn::Left,
                (false, true) => Turn::Right,
                (true, true) => Turn::None,
            };

            /* Clear events */
            gpiote.channel0().reset_events();
            gpiote.channel1().reset_events();

            //rprintln!("Turn: {:?}", turn)

            *TURN.borrow(cs).borrow_mut() = turn;
        }
    });
}
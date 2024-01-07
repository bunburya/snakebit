use core::cmp::{max, min};
use heapless::FnvIndexSet;
use heapless::spsc::Queue;
use microbit::hal::Rng;
use crate::control::Turn;

/// Number of rows in our grid (ie, our LED matrix)
const N_ROWS: usize = 5;
/// Number of columns in our grid
const N_COLS: usize = 5;

type CoordSet = FnvIndexSet<Coords, 32>;

/// Define the directions the snake can move
enum Direction {
    Up,
    Down,
    Left,
    Right
}


pub enum GameStatus {
    Won,
    Lost,
    Ongoing
}

/// The outcome of a single move/step.
enum StepOutcome {
    /// Grid full (player wins)
    Full(Coords),
    /// Snake has collided with itself (player loses)
    Collision(Coords),
    /// Snake has left the edge of the grid (player loses)
    OutOfBounds(Coords),
    /// Snake has eaten some food
    Eat(Coords),
    /// Snake has moved (and nothing else has happened)
    Move(Coords)
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
struct Coords {
    // Signed ints to allow negative values (handy when checking if we have gone off the top or left
    // of the grid)
    row: i8,
    col: i8
}

impl Coords {

    /// Get random coordinates within a grid. `exclude` is an optional set of coordinates which
    /// should be excluded from the output.
    fn random(
        rng: &mut Rng,
        exclude: Option<&CoordSet>
    ) -> Self {
        let mut coords = Coords {
            row: ((rng.random_u8() as usize) % N_ROWS) as i8,
            col: ((rng.random_u8() as usize) % N_COLS) as i8
        };
        while exclude.is_some_and(|exc| exc.contains(&coords)) {
            coords = Coords {
                row: ((rng.random_u8() as usize) % N_ROWS) as i8,
                col: ((rng.random_u8() as usize) % N_COLS) as i8
            }
        }
        coords
    }

    fn is_out_of_bounds(&self) -> bool {
        self.row < 0 || self.row >= (N_ROWS as i8) || self.col < 0 || self.col >= (N_COLS as i8)
    }
}

struct Snake {
    /// Coordinates of the snake's head.
    head: Coords,
    /// Queue of coordinates of the rest of the snake's body. The end of the tail is at the front.
    tail: Queue<Coords, 32>,
    /// A set containing all coordinates currently occupied by the snake (for fast collision
    /// checking).
    coord_set: CoordSet,
    /// The direction the snake is currently moving in.
    direction: Direction
}

impl Snake {
    fn new() -> Self {
        let head = Coords { row: 2, col: 2 };
        let initial_tail = Coords { row: 2, col: 1 };
        let mut tail = Queue::new();
        tail.enqueue(initial_tail).unwrap();
        let mut coord_set: CoordSet = FnvIndexSet::new();
        coord_set.insert(head).unwrap();
        coord_set.insert(initial_tail).unwrap();
        Snake {
            head,
            tail,
            coord_set,
            direction: Direction::Right,
        }
    }

    /// Move the snake onto the given coordinates. If `extend` is false, the snake's tail vacates
    /// the rearmost tile.
    fn move_snake(&mut self, coords: Coords, extend: bool) {
        // Location of head becomes front of tail
        self.tail.enqueue(self.head).unwrap();
        // Head moves to new coords
        self.head = coords;

        self.coord_set.insert(coords).unwrap();

        if !extend {
            let back = self.tail.dequeue().unwrap();
            self.coord_set.remove(&back);
        }
    }

    fn turn_right(&mut self) {
        self.direction = match self.direction {
            Direction::Up => Direction::Right,
            Direction::Down => Direction::Left,
            Direction::Left => Direction::Up,
            Direction::Right => Direction::Down
        }
    }

    fn turn_left(&mut self) {
        self.direction = match self.direction {
            Direction::Up => Direction::Left,
            Direction::Down => Direction::Right,
            Direction::Left => Direction::Down,
            Direction::Right => Direction::Up
        }
    }

    fn turn(&mut self, direction: Turn) {
        match direction {
            Turn::Left => self.turn_left(),
            Turn::Right => self.turn_right(),
            Turn::None => ()
        }
    }
}

/// Struct to hold game state and associated behaviour
pub(crate) struct Game {
    rng: Rng,
    snake: Snake,
    food_coords: Coords,
    speed: u8,
    pub(crate) status: GameStatus,
    score: u8
}

impl Game {

    pub(crate) fn new(mut rng: Rng) -> Self {
        let mut tail: CoordSet = FnvIndexSet::new();
        tail.insert(Coords { row: 2, col: 1 }).unwrap();
        let snake = Snake::new();
        let food_coords = Coords::random(&mut rng, Some(&snake.coord_set));
        Game {
            rng,
            snake,
            food_coords,
            speed: 1,
            status: GameStatus::Ongoing,
            score: 0
        }
    }

    /// Reset the game state to start a new game.
    pub(crate) fn reset(&mut self) {
        self.snake = Snake::new();
        self.place_food();
        self.speed = 1;
        self.status = GameStatus::Ongoing;
        self.score = 0;
    }

    /// Randomly place food on the grid.
    fn place_food(&mut self) -> Coords {
        let coords = Coords::random(&mut self.rng, Some(&self.snake.coord_set));
        self.food_coords = coords;
        coords
    }

    /// Assess the snake's next move and return the outcome. Doesn't actually update the game state.
    fn get_step_outcome(&self) -> StepOutcome {
        let head = &self.snake.head;
        let next_move = match self.snake.direction {
            Direction::Up => Coords { row: head.row - 1, col: head.col },
            Direction::Down => Coords { row: head.row + 1, col: head.col },
            Direction::Left => Coords { row: head.row, col: head.col - 1 },
            Direction::Right => Coords { row: head.row, col: head.col + 1 },
        };
        if next_move.is_out_of_bounds() {
            StepOutcome::OutOfBounds(next_move)
        } else if self.snake.coord_set.contains(&next_move) {
            // We haven't moved the snake yet, so if the next move is at the end of the tail, there
            // won't actually be any collision (as the tail will have moved by the time the head
            // moves onto the tile)
            if next_move != *self.snake.tail.peek().unwrap() {
                StepOutcome::Collision(next_move)
            } else {
                StepOutcome::Move(next_move)
            }
        } else if next_move == self.food_coords {
            if self.snake.tail.len() == 23 {
                StepOutcome::Full(next_move)
            } else {
                StepOutcome::Eat(next_move)
            }
        } else {
            StepOutcome::Move(next_move)
        }
    }

    /// Handle the outcome of a step, updating the game's internal state.
    fn handle_step_outcome(&mut self, outcome: StepOutcome) {
        self.status = match outcome {
            StepOutcome::OutOfBounds(_) => GameStatus::Lost,
            StepOutcome::Collision(_) => GameStatus::Lost,
            StepOutcome::Full(_) => GameStatus::Won,
            StepOutcome::Eat(c) => {
                self.snake.move_snake(c, true);
                self.place_food();
                self.score += 1;
                if self.score % 5 == 0 {
                    self.speed += 1
                }
                GameStatus::Ongoing
            },
            StepOutcome::Move(c) => {
                self.snake.move_snake(c, false);
                GameStatus::Ongoing
            }
        }
    }


    pub(crate) fn step(&mut self, turn: Turn) {
        self.snake.turn(turn);
        let outcome = self.get_step_outcome();
        self.handle_step_outcome(outcome);
    }

    /// Calculate the length of time to wait between game steps, in milliseconds. Generally this
    /// will get lower as the player's score increases, but need to be careful it cannot result in a
    /// value below zero.
    pub(crate) fn step_len_ms(&self) -> u32 {
        let result = 1000 - (200 * ((self.speed as i32) - 1));
        max(result, 200) as u32
    }

    /// Return an array representing the game state, which can be used to display the state on the
    /// microbit's LED matrix.
    pub(crate) fn game_matrix(
        &self,
        head_brightness: u8,
        tail_brightness: u8,
        food_brightness: u8
    ) -> [[u8; N_COLS]; N_ROWS] {
        let mut values = [[0u8; N_COLS]; N_ROWS];
        values[self.snake.head.row as usize][self.snake.head.col as usize] = head_brightness;
        for t in &self.snake.tail {
            values[t.row as usize][t.col as usize] = tail_brightness
        }
        values[self.food_coords.row as usize][self.food_coords.col as usize] = food_brightness;
        values
    }

    /// Return an array representing the game score, which can be used to display the score on the
    /// microbit's LED matrix (by illuminating the equivalent number of LEDs, going left->right and
    /// top->bottom).
    pub(crate) fn score_matrix(&self, brightness: u8) -> [[u8; N_COLS]; N_ROWS] {
        let mut values = [[0u8; N_COLS]; N_ROWS];
        let full_rows = (self.score as usize) / N_COLS;
        for r in 0..full_rows {
            values[r] = [brightness; N_COLS];
        }
        for c in 0..(self.score as usize) % N_COLS {
            values[full_rows][c] = brightness;
        }
        values
    }
}
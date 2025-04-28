use crate::screen::{Writer, screenwriter};
use alloc::format;
use core::fmt::Write;
use core::sync::atomic::{AtomicBool, AtomicI32, Ordering};

// Game dimensions and constants
const SCREEN_WIDTH: usize = 640;
const SCREEN_HEIGHT: usize = 480;
const PADDLE_WIDTH: usize = 10;
const PADDLE_HEIGHT: usize = 60;
const BALL_SIZE: usize = 10;
const PADDLE_OFFSET: usize = 20;
const PADDLE_SPEED: i32 = 5;
const INITIAL_BALL_SPEED_X: i32 = 2;
const INITIAL_BALL_SPEED_Y: i32 = 2;

// Game state using atomics for thread safety
static LEFT_PADDLE_Y: AtomicI32 = AtomicI32::new((SCREEN_HEIGHT as i32 - PADDLE_HEIGHT as i32) / 2);
static RIGHT_PADDLE_Y: AtomicI32 = AtomicI32::new((SCREEN_HEIGHT as i32 - PADDLE_HEIGHT as i32) / 2);
static BALL_X: AtomicI32 = AtomicI32::new((SCREEN_WIDTH as i32 - BALL_SIZE as i32) / 2);
static BALL_Y: AtomicI32 = AtomicI32::new((SCREEN_HEIGHT as i32 - BALL_SIZE as i32) / 2);
static BALL_VEL_X: AtomicI32 = AtomicI32::new(INITIAL_BALL_SPEED_X);
static BALL_VEL_Y: AtomicI32 = AtomicI32::new(INITIAL_BALL_SPEED_Y);
static LEFT_SCORE: AtomicI32 = AtomicI32::new(0);
static RIGHT_SCORE: AtomicI32 = AtomicI32::new(0);
static GAME_ACTIVE: AtomicBool = AtomicBool::new(false);

// Add key state tracking
static KEY_W_PRESSED: AtomicBool = AtomicBool::new(false);
static KEY_S_PRESSED: AtomicBool = AtomicBool::new(false);

// Add simulation key release timer
static KEY_RELEASE_TIMER: AtomicI32 = AtomicI32::new(0);
const KEY_RELEASE_DELAY: i32 = 5; // Auto-release keys after this many ticks

// Add state for right paddle oscillation
static RIGHT_PADDLE_DIRECTION: AtomicI32 = AtomicI32::new(1); // 1 = down, -1 = up
const RIGHT_PADDLE_SPEED: i32 = 3; // Speed for automatic movement

pub fn init_game() {
    // Reset game state
    LEFT_PADDLE_Y.store((SCREEN_HEIGHT as i32 - PADDLE_HEIGHT as i32) / 2, Ordering::SeqCst);
    RIGHT_PADDLE_Y.store((SCREEN_HEIGHT as i32 - PADDLE_HEIGHT as i32) / 2, Ordering::SeqCst);
    BALL_X.store((SCREEN_WIDTH as i32 - BALL_SIZE as i32) / 2, Ordering::SeqCst);
    BALL_Y.store((SCREEN_HEIGHT as i32 - BALL_SIZE as i32) / 2, Ordering::SeqCst);
    BALL_VEL_X.store(INITIAL_BALL_SPEED_X, Ordering::SeqCst);
    BALL_VEL_Y.store(INITIAL_BALL_SPEED_Y, Ordering::SeqCst);
    LEFT_SCORE.store(0, Ordering::SeqCst);
    RIGHT_SCORE.store(0, Ordering::SeqCst);
    GAME_ACTIVE.store(true, Ordering::SeqCst);
    
    // Initialize key states
    KEY_W_PRESSED.store(false, Ordering::SeqCst);
    KEY_S_PRESSED.store(false, Ordering::SeqCst);
    
    // Initialize oscillation direction for right paddle
    RIGHT_PADDLE_DIRECTION.store(1, Ordering::SeqCst);
    
    // Display initial game state
    draw_game();
    
    // Show instructions
    write!(Writer, "\n\nControls:\n").unwrap();
    write!(Writer, "W/S: Move left paddle\n").unwrap();
    write!(Writer, "Press SPACE to start\n").unwrap();
}

// Set key state functions
pub fn set_key_w(pressed: bool) {
    KEY_W_PRESSED.store(pressed, Ordering::SeqCst);
    if pressed {
        KEY_RELEASE_TIMER.store(0, Ordering::SeqCst);
    }
}

pub fn set_key_s(pressed: bool) {
    KEY_S_PRESSED.store(pressed, Ordering::SeqCst);
    if pressed {
        KEY_RELEASE_TIMER.store(0, Ordering::SeqCst);
    }
}

pub fn start_game() {
    GAME_ACTIVE.store(true, Ordering::SeqCst);
}

pub fn move_left_paddle_up() {
    if GAME_ACTIVE.load(Ordering::SeqCst) {
        let current = LEFT_PADDLE_Y.load(Ordering::SeqCst);
        if current > PADDLE_SPEED {
            LEFT_PADDLE_Y.store(current - PADDLE_SPEED, Ordering::SeqCst);
        } else {
            LEFT_PADDLE_Y.store(0, Ordering::SeqCst);
        }
    }
}

pub fn move_left_paddle_down() {
    if GAME_ACTIVE.load(Ordering::SeqCst) {
        let current = LEFT_PADDLE_Y.load(Ordering::SeqCst);
        if current < (SCREEN_HEIGHT as i32 - PADDLE_HEIGHT as i32) - PADDLE_SPEED {
            LEFT_PADDLE_Y.store(current + PADDLE_SPEED, Ordering::SeqCst);
        } else {
            LEFT_PADDLE_Y.store(SCREEN_HEIGHT as i32 - PADDLE_HEIGHT as i32, Ordering::SeqCst);
        }
    }
}

pub fn update_game() {
    if !GAME_ACTIVE.load(Ordering::SeqCst) {
        return;
    }
    
    // Auto-release key simulation
    let timer = KEY_RELEASE_TIMER.fetch_add(1, Ordering::SeqCst);
    if timer >= KEY_RELEASE_DELAY {
        KEY_RELEASE_TIMER.store(0, Ordering::SeqCst);
        
        // Auto-release all keys - only for left paddle now
        KEY_W_PRESSED.store(false, Ordering::SeqCst);
        KEY_S_PRESSED.store(false, Ordering::SeqCst);
    }
    
    // Check for active key states and move left paddle accordingly
    if KEY_W_PRESSED.load(Ordering::SeqCst) {
        move_left_paddle_up();
    }
    if KEY_S_PRESSED.load(Ordering::SeqCst) {
        move_left_paddle_down();
    }
    
    // Automatically oscillate right paddle
    let right_paddle_y = RIGHT_PADDLE_Y.load(Ordering::SeqCst);
    let right_paddle_dir = RIGHT_PADDLE_DIRECTION.load(Ordering::SeqCst);
    
    // Check if we need to change direction
    if right_paddle_y <= 0 {
        RIGHT_PADDLE_DIRECTION.store(1, Ordering::SeqCst);
    } else if right_paddle_y >= SCREEN_HEIGHT as i32 - PADDLE_HEIGHT as i32 {
        RIGHT_PADDLE_DIRECTION.store(-1, Ordering::SeqCst);
    }
    
    // Move the paddle based on current direction
    if right_paddle_dir > 0 {
        // Move down
        if right_paddle_y < SCREEN_HEIGHT as i32 - PADDLE_HEIGHT as i32 - RIGHT_PADDLE_SPEED {
            RIGHT_PADDLE_Y.store(right_paddle_y + RIGHT_PADDLE_SPEED, Ordering::SeqCst);
        } else {
            RIGHT_PADDLE_Y.store(SCREEN_HEIGHT as i32 - PADDLE_HEIGHT as i32, Ordering::SeqCst);
            RIGHT_PADDLE_DIRECTION.store(-1, Ordering::SeqCst);
        }
    } else {
        // Move up
        if right_paddle_y > RIGHT_PADDLE_SPEED {
            RIGHT_PADDLE_Y.store(right_paddle_y - RIGHT_PADDLE_SPEED, Ordering::SeqCst);
        } else {
            RIGHT_PADDLE_Y.store(0, Ordering::SeqCst);
            RIGHT_PADDLE_DIRECTION.store(1, Ordering::SeqCst);
        }
    }
    
    // Move ball
    let mut ball_x = BALL_X.load(Ordering::SeqCst);
    let mut ball_y = BALL_Y.load(Ordering::SeqCst);
    let mut vel_x = BALL_VEL_X.load(Ordering::SeqCst);
    let mut vel_y = BALL_VEL_Y.load(Ordering::SeqCst);
    
    ball_x += vel_x;
    ball_y += vel_y;
    
    // Check for collisions with top/bottom walls
    if ball_y <= 0 || ball_y >= SCREEN_HEIGHT as i32 - BALL_SIZE as i32 {
        vel_y = -vel_y;
    }
    
    // Check for collisions with paddles
    let left_paddle_y = LEFT_PADDLE_Y.load(Ordering::SeqCst);
    
    // Left paddle collision
    if ball_x <= PADDLE_OFFSET as i32 + PADDLE_WIDTH as i32 && 
       ball_x >= PADDLE_OFFSET as i32 &&
       ball_y + BALL_SIZE as i32 >= left_paddle_y && 
       ball_y <= left_paddle_y + PADDLE_HEIGHT as i32 {
        ball_x = PADDLE_OFFSET as i32 + PADDLE_WIDTH as i32;
        vel_x = -vel_x;
        // Increase velocity slightly for difficulty
        if vel_x < 0 { vel_x -= 1; } else { vel_x += 1; }
    }
    
    // Right paddle collision
    if ball_x + BALL_SIZE as i32 >= SCREEN_WIDTH as i32 - PADDLE_OFFSET as i32 - PADDLE_WIDTH as i32 && 
       ball_x + BALL_SIZE as i32 <= SCREEN_WIDTH as i32 - PADDLE_OFFSET as i32 &&
       ball_y + BALL_SIZE as i32 >= right_paddle_y && 
       ball_y <= right_paddle_y + PADDLE_HEIGHT as i32 {
        ball_x = SCREEN_WIDTH as i32 - PADDLE_OFFSET as i32 - PADDLE_WIDTH as i32 - BALL_SIZE as i32;
        vel_x = -vel_x;
        // Increase velocity slightly for difficulty
        if vel_x < 0 { vel_x -= 1; } else { vel_x += 1; }
    }
    
    // Check for scoring
    if ball_x <= 0 {
        // Right player scores
        RIGHT_SCORE.fetch_add(1, Ordering::SeqCst);
        reset_ball();
        draw_scores();
        return;
    }
    
    if ball_x >= SCREEN_WIDTH as i32 - BALL_SIZE as i32 {
        // Left player scores
        LEFT_SCORE.fetch_add(1, Ordering::SeqCst);
        reset_ball();
        draw_scores();
        return;
    }
    
    // Update ball state
    BALL_X.store(ball_x, Ordering::SeqCst);
    BALL_Y.store(ball_y, Ordering::SeqCst);
    BALL_VEL_X.store(vel_x, Ordering::SeqCst);
    BALL_VEL_Y.store(vel_y, Ordering::SeqCst);
    
    draw_game();
}

fn reset_ball() {
    BALL_X.store((SCREEN_WIDTH as i32 - BALL_SIZE as i32) / 2, Ordering::SeqCst);
    BALL_Y.store((SCREEN_HEIGHT as i32 - BALL_SIZE as i32) / 2, Ordering::SeqCst);
    BALL_VEL_X.store(if BALL_VEL_X.load(Ordering::SeqCst) < 0 { INITIAL_BALL_SPEED_X } else { -INITIAL_BALL_SPEED_X }, Ordering::SeqCst);
    BALL_VEL_Y.store(INITIAL_BALL_SPEED_Y, Ordering::SeqCst);
}

fn draw_scores() {
    let left_score = LEFT_SCORE.load(Ordering::SeqCst);
    let right_score = RIGHT_SCORE.load(Ordering::SeqCst);
    
    // Clear score area
    for x in 0..SCREEN_WIDTH {
        for y in 5..30 {
            screenwriter().draw_pixel(x, y, 0, 0, 0);
        }
    }
    
    // Draw score text
    let score_text = format!("Score: {} - {}", left_score, right_score);
    write!(Writer, "\r{}", score_text).unwrap();
}

fn draw_game() {
    // Clear screen (except text area)
    for y in 30..SCREEN_HEIGHT {
        for x in 0..SCREEN_WIDTH {
            screenwriter().draw_pixel(x, y, 0, 0, 0);
        }
    }
    
    // Draw center line
    for y in 30..SCREEN_HEIGHT {
        if y % 8 < 4 {
            screenwriter().draw_pixel(SCREEN_WIDTH / 2, y, 255, 255, 255);
        }
    }
    
    // Draw paddles
    let left_paddle_y = LEFT_PADDLE_Y.load(Ordering::SeqCst) as usize;
    let right_paddle_y = RIGHT_PADDLE_Y.load(Ordering::SeqCst) as usize;
    
    // Left paddle
    for y in left_paddle_y..left_paddle_y + PADDLE_HEIGHT {
        for x in PADDLE_OFFSET..PADDLE_OFFSET + PADDLE_WIDTH {
            if y < SCREEN_HEIGHT && x < SCREEN_WIDTH {
                screenwriter().draw_pixel(x, y, 255, 255, 255);
            }
        }
    }
    
    // Right paddle
    for y in right_paddle_y..right_paddle_y + PADDLE_HEIGHT {
        for x in (SCREEN_WIDTH - PADDLE_OFFSET - PADDLE_WIDTH)..(SCREEN_WIDTH - PADDLE_OFFSET) {
            if y < SCREEN_HEIGHT && x < SCREEN_WIDTH {
                screenwriter().draw_pixel(x, y, 255, 255, 255);
            }
        }
    }
    
    // Draw ball
    let ball_x = BALL_X.load(Ordering::SeqCst) as usize;
    let ball_y = BALL_Y.load(Ordering::SeqCst) as usize;
    
    for y in ball_y..ball_y + BALL_SIZE {
        for x in ball_x..ball_x + BALL_SIZE {
            if y < SCREEN_HEIGHT && x < SCREEN_WIDTH {
                screenwriter().draw_pixel(x, y, 255, 255, 255);
            }
        }
    }
    
    // Draw scores
    draw_scores();
}

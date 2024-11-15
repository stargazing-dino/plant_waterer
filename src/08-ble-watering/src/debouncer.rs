use embassy_nrf::gpio::{Input, Level};
use embassy_time::{Duration, Timer};

pub struct Debouncer<'a> {
    input: Input<'a>,
    debounce: Duration,
}

impl<'a> Debouncer<'a> {
    pub fn new(input: Input<'a>, debounce: Duration) -> Self {
        Self { input, debounce }
    }

    /// Debounces the input signal by waiting for a stable level.
    ///
    /// This method continuously checks the input level and waits for any edge.
    /// After detecting an edge, it waits for the specified debounce duration
    /// and checks the input level again. If the level has changed, it returns
    /// the new level.
    ///
    /// # Returns
    ///
    /// * `Level` - The stable level of the input signal after debouncing.
    pub async fn debounce(&mut self) -> Level {
        loop {
            let l1 = self.input.get_level();

            self.input.wait_for_any_edge().await;

            Timer::after(self.debounce).await;

            let l2 = self.input.get_level();
            if l1 != l2 {
                break l2;
            }
        }
    }
}

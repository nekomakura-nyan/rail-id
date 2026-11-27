//! Cached 100 ms timestamp clock.
//!
//! The RailID timestamp has 100 ms resolution. Calling `SystemTime::now` for
//! every ID is unnecessary within one tick, so the default single-ID path uses
//! a thread-local clock. Bulk workers use one clock per worker.
//!
//! The cache is bounded to 100 ms and never moves backwards if the wall clock is
//! adjusted backwards. Explicit-time APIs (`generate_with_profile_and_time` and
//! `generate_with_source`) are unaffected.

use std::cell::RefCell;
use std::time::{Duration, Instant, SystemTime};

use crate::{ticks_from_system_time, RailIdError};

const TICK_DURATION: Duration = Duration::from_millis(100);

pub(crate) struct TickClock {
    last_refresh: Instant,
    ticks: u64,
}

impl TickClock {
    pub(crate) fn new() -> Result<Self, RailIdError> {
        let ticks = ticks_from_system_time(SystemTime::now())?;
        Ok(Self {
            last_refresh: Instant::now(),
            ticks,
        })
    }

    pub(crate) fn now(&mut self) -> Result<u64, RailIdError> {
        let now = Instant::now();
        if now.saturating_duration_since(self.last_refresh) < TICK_DURATION {
            return Ok(self.ticks);
        }

        let ticks = ticks_from_system_time(SystemTime::now())?;
        // Never emit a timestamp older than one already emitted by this clock.
        self.ticks = self.ticks.max(ticks);
        self.last_refresh = now;
        Ok(self.ticks)
    }
}

thread_local! {
    static CLOCK: RefCell<Option<TickClock>> = const { RefCell::new(None) };
}

pub(crate) fn current_ticks() -> Result<u64, RailIdError> {
    CLOCK.with(|cell| {
        let mut slot = cell.borrow_mut();
        if slot.is_none() {
            *slot = Some(TickClock::new()?);
        }
        slot.as_mut().ok_or(RailIdError::TimestampOutOfRange)?.now()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cached_clock_does_not_go_backwards_within_a_tick() {
        let mut clock = TickClock::new().expect("clock");
        let first = clock.now().expect("first tick");
        let second = clock.now().expect("second tick");
        assert_eq!(first, second);
    }
}

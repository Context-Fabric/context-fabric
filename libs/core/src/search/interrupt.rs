//! Stopping a running search from outside: the join polls an [`Interrupt`]
//! while it works, and stops with [`CfError::Interrupted`] once it reports
//! an interruption.

use std::time::{Duration, Instant};

use crate::error::{CfError, Result};

/// A source of interruptions, such as pending Ctrl-C signals.
pub trait Interrupt: Sync {
    /// Whether the running search should stop now.
    fn interrupted(&self) -> bool;
}

/// Candidate steps between clock reads.
const STEPS_PER_CLOCK_READ: u32 = 4096;

/// The time between two polls of the [`Interrupt`].
const POLL_INTERVAL: Duration = Duration::from_millis(50);

/// Polls an [`Interrupt`] about every [`POLL_INTERVAL`] of join work, so that
/// polling costs a counter increment per step.
pub(crate) struct InterruptPoll<'i> {
    interrupt: Option<&'i dyn Interrupt>,
    steps: u32,
    last_poll: Instant,
}

impl<'i> InterruptPoll<'i> {
    pub(crate) fn new(interrupt: Option<&'i dyn Interrupt>) -> Self {
        Self {
            interrupt,
            steps: 0,
            last_poll: Instant::now(),
        }
    }

    /// Count one unit of join work; fails with [`CfError::Interrupted`] when
    /// a due poll reports an interruption.
    pub(crate) fn step(&mut self) -> Result<()> {
        let Some(interrupt) = self.interrupt else {
            return Ok(());
        };
        self.steps += 1;
        if self.steps < STEPS_PER_CLOCK_READ {
            return Ok(());
        }
        self.steps = 0;
        if self.last_poll.elapsed() < POLL_INTERVAL {
            return Ok(());
        }
        self.last_poll = Instant::now();
        if interrupt.interrupted() {
            return Err(CfError::Interrupted);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, Ordering};

    use super::*;

    struct Flag(AtomicBool);

    impl Interrupt for Flag {
        fn interrupted(&self) -> bool {
            self.0.load(Ordering::Relaxed)
        }
    }

    #[test]
    fn a_raised_flag_stops_work_after_the_poll_interval() {
        let flag = Flag(AtomicBool::new(true));
        let mut poll = InterruptPoll::new(Some(&flag));
        poll.last_poll = Instant::now() - POLL_INTERVAL;
        let stopped = (0..STEPS_PER_CLOCK_READ).find_map(|_| poll.step().err());
        assert!(matches!(stopped, Some(CfError::Interrupted)));
    }

    #[test]
    fn without_an_interrupt_work_never_stops() {
        let mut poll = InterruptPoll::new(None);
        assert!((0..3 * STEPS_PER_CLOCK_READ).all(|_| poll.step().is_ok()));
    }
}

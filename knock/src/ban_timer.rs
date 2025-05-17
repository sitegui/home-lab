use chrono::{DateTime, TimeDelta, Utc};
use serde::{Deserialize, Serialize};

/// A helper struct to apply bans after some failed attempts
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct BanTimer {
    failures: u16,
    banned_until: Option<DateTime<Utc>>,
}

/// If the method [`Attempting::report_success()`] is not called, a failure will be reported.
#[derive(Debug)]
pub struct Attempting<'a> {
    now: DateTime<Utc>,
    timer: &'a mut BanTimer,
    max_failures: u16,
    ban_duration: TimeDelta,
    reported_success: bool,
}

impl BanTimer {
    pub fn attempt(
        &mut self,
        now: DateTime<Utc>,
        max_failures: u16,
        ban_duration: TimeDelta,
    ) -> Option<Attempting> {
        match self.banned_until {
            Some(banned_until) if banned_until > now => None,
            _ => Some(Attempting {
                now,
                timer: self,
                max_failures,
                ban_duration,
                reported_success: false,
            }),
        }
    }
}

impl Attempting<'_> {
    pub fn report_success(mut self) {
        self.timer.failures = 0;
        self.timer.banned_until = None;
        self.reported_success = true;
    }
}

impl Drop for Attempting<'_> {
    fn drop(&mut self) {
        if !self.reported_success {
            self.timer.failures += 1;
            if self.timer.failures >= self.max_failures {
                self.timer.failures = 0;
                self.timer.banned_until = Some(self.now + self.ban_duration);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        let mut timer = BanTimer::default();
        let start = Utc::now();
        let max_failures = 2;
        let ban_duration = TimeDelta::seconds(10);

        drop(timer.attempt(start, max_failures, ban_duration).unwrap());
        drop(timer.attempt(start, max_failures, ban_duration).unwrap());
        assert!(timer.attempt(start, max_failures, ban_duration).is_none());

        drop(
            timer
                .attempt(start + ban_duration, max_failures, ban_duration)
                .unwrap(),
        );
        timer
            .attempt(start + ban_duration, max_failures, ban_duration)
            .unwrap()
            .report_success();
        drop(
            timer
                .attempt(start + ban_duration, max_failures, ban_duration)
                .unwrap(),
        );
        assert_eq!(timer.failures, 1);
    }
}

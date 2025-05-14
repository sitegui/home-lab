use chrono::{DateTime, TimeDelta, Utc};
use serde::{Deserialize, Serialize};

/// A helper struct to apply bans after some failed attempts
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct BanTimer {
    failures: u16,
    banned_until: Option<DateTime<Utc>>,
}

#[derive(Debug)]
pub struct Attempting<'a> {
    now: DateTime<Utc>,
    timer: &'a mut BanTimer,
}

impl BanTimer {
    pub fn attempt(&mut self, now: DateTime<Utc>) -> Option<Attempting> {
        match self.banned_until {
            Some(banned_until) if banned_until > now => None,
            _ => Some(Attempting { now, timer: self }),
        }
    }
}

impl Attempting<'_> {
    pub fn report_success(self) {
        self.timer.failures = 0;
        self.timer.banned_until = None;
    }

    pub fn report_failure(self, max_failures: u16, ban_duration: TimeDelta) {
        self.timer.failures += 1;
        if self.timer.failures >= max_failures {
            self.timer.failures = 0;
            self.timer.banned_until = Some(self.now + ban_duration);
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

        timer
            .attempt(start)
            .unwrap()
            .report_failure(max_failures, ban_duration);
        timer
            .attempt(start)
            .unwrap()
            .report_failure(max_failures, ban_duration);
        assert!(timer.attempt(start).is_none());

        timer
            .attempt(start + ban_duration)
            .unwrap()
            .report_failure(max_failures, ban_duration);
        timer
            .attempt(start + ban_duration)
            .unwrap()
            .report_success();
        timer
            .attempt(start + ban_duration)
            .unwrap()
            .report_failure(max_failures, ban_duration);
        assert_eq!(timer.failures, 1);
    }
}

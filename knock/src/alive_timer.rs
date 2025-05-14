use chrono::{DateTime, TimeDelta, Utc};
use serde::{Deserialize, Serialize};

/// A helper struct that is used to detect when something like a session is no longer valid.
///
/// A [`AliveTimer`] becomes dead if at least one of these conditions is met:
/// 1. expiration: the struct was created more than `max_lifetime` ago
/// 2. inactivity: the last call to [`AliveTimer::new()`] or [`AliveTimer::check_alive()`] was
///    more than `max_inactivity` ago
#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct AliveTimer {
    created_at: DateTime<Utc>,
    last_activity: DateTime<Utc>,
}

impl AliveTimer {
    pub fn new(now: DateTime<Utc>) -> Self {
        Self {
            created_at: now,
            last_activity: now,
        }
    }

    /// Returns `false` if this timer is dead. If still alive, update the last activity marker.
    pub fn check_alive(
        &mut self,
        now: DateTime<Utc>,
        max_lifetime: TimeDelta,
        max_inactivity: TimeDelta,
    ) -> bool {
        if self.is_alive(now, max_lifetime, max_inactivity) {
            self.last_activity = now;
            true
        } else {
            false
        }
    }

    /// Unlike [`AliveTimer::check_alive()`], this will not update its last activity marker.
    pub fn is_alive(
        &self,
        now: DateTime<Utc>,
        max_lifetime: TimeDelta,
        max_inactivity: TimeDelta,
    ) -> bool {
        now < self.created_at + max_lifetime && now < self.last_activity + max_inactivity
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        let start = Utc::now();
        let max_lifetime = TimeDelta::seconds(3600);
        let max_inactivity = TimeDelta::seconds(60);
        let dt = TimeDelta::seconds(1);

        let mut timer = AliveTimer::new(start);

        let check_time_1 = start + dt;
        assert!(timer.is_alive(check_time_1, max_lifetime, max_inactivity));
        assert_eq!(timer.last_activity, start);

        assert!(timer.check_alive(check_time_1, max_lifetime, max_inactivity));
        assert_eq!(timer.last_activity, check_time_1);

        let check_time_2 = start + dt * 2 + max_inactivity;
        assert!(!timer.is_alive(check_time_2, max_lifetime, max_inactivity));
        assert_eq!(timer.last_activity, check_time_1);

        assert!(!timer.check_alive(check_time_2, max_lifetime, max_inactivity));
        assert_eq!(timer.last_activity, check_time_1);
    }
}

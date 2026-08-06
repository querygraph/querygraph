use std::sync::{Arc, Mutex};

use chrono::{DateTime, TimeDelta, Utc};

use marciana_cognition::CognitionClock;

#[derive(Clone)]
pub(crate) struct TestClock {
    now: Arc<Mutex<DateTime<Utc>>>,
}

impl TestClock {
    pub(crate) fn new(now: DateTime<Utc>) -> Self {
        Self {
            now: Arc::new(Mutex::new(now)),
        }
    }

    pub(crate) fn now(&self) -> DateTime<Utc> {
        *self.now.lock().expect("test clock lock")
    }

    pub(crate) fn advance(&self, delta: TimeDelta) {
        let mut now = self.now.lock().expect("test clock lock");
        *now += delta;
    }
}

impl CognitionClock for TestClock {
    fn now(&self) -> DateTime<Utc> {
        self.now()
    }
}

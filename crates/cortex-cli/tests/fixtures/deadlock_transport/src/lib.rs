//! Intentional lock-ordering smell for A2A consensus integration tests.

use std::sync::Mutex;

static A: Mutex<()> = Mutex::new(());
static B: Mutex<()> = Mutex::new(());

pub fn naive_spin_lock_path() {
    let _a = A.lock().unwrap();
    let _b = B.lock().unwrap();
}

pub fn ordered_mutex_path() {
    let _b = B.lock().unwrap();
    let _a = A.lock().unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordered_locks_do_not_panic() {
        ordered_mutex_path();
    }
}

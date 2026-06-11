//! Cyclic `tokio::sync::Mutex` lock order — intentional deadlock smell for A2A consensus tests.

use std::sync::Arc;
use tokio::sync::Mutex;

static CHANNEL_A: Mutex<()> = Mutex::const_new(());
static CHANNEL_B: Mutex<()> = Mutex::const_new(());

/// Naive path: lock A then B (conflicts with path that locks B then A).
pub async fn naive_spin_lock_path() {
    let _a = CHANNEL_A.lock().await;
    let _b = CHANNEL_B.lock().await;
}

/// Ordered path: consistent lock order (B then A) avoids AB-BA deadlock.
pub async fn ordered_mutex_path() {
    let _b = CHANNEL_B.lock().await;
    let _a = CHANNEL_A.lock().await;
}

/// Cross-task cycle: task1 holds A and waits for B; task2 holds B and waits for A.
pub async fn cyclic_deadlock_demo() {
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));

    let a1 = a.clone();
    let b1 = b.clone();
    let t1 = tokio::spawn(async move {
        let _ga = a1.lock().await;
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        let _gb = b1.lock().await;
    });

    let a2 = a.clone();
    let b2 = b.clone();
    let t2 = tokio::spawn(async move {
        let _gb = b2.lock().await;
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        let _ga = a2.lock().await;
    });

    let _ = tokio::join!(t1, t2);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn ordered_mutex_path_completes() {
        ordered_mutex_path().await;
    }
}

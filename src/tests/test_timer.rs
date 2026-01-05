use crate::timer::Timer;

#[test]
fn test_timer_completion() {
    let mut timer = Timer::new(1.0);
    assert!(!timer.is_finished());

    timer.update(0.5);
    assert!(!timer.is_finished());

    timer.update(0.6); // Total 1.1s
    assert!(timer.is_finished());
}

#[test]
fn test_timer_reset() {
    let mut timer = Timer::new(1.0);
    timer.update(1.5);
    timer.reset();
    assert!(!timer.is_finished());
    assert_eq!(timer.elapsed, 0.0);
}

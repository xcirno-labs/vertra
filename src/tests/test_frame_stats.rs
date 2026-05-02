use std::thread::sleep;
use std::time::{Duration, Instant};
use crate::frame_stats::{FrameStats, SAMPLE_WINDOW_SECS_DEFAULT};

fn make_stats() -> FrameStats {
    FrameStats::new()
}

#[test]
fn no_commit_before_half_second_window() {
    let mut stats = make_stats();

    for _ in 0..10 {
        stats.tick(0.020);
    }

    assert_eq!(stats.fps, 0.0);
    assert_eq!(stats.frame_time_ms, 0.0);
}

#[test]
fn commit_happens_once_half_second_window_is_reached() {
    let mut stats = make_stats();

    let start = Instant::now();
    while start.elapsed().as_secs_f32() < SAMPLE_WINDOW_SECS_DEFAULT + 0.05 {
        stats.tick(1.0 / 60.0);
        sleep(Duration::from_millis(16));
    }

    assert!(stats.fps > 0.0, "expected a committed fps sample, got {}", stats.fps);
    assert!(
        stats.frame_time_ms > 0.0,
        "expected a committed frame time sample, got {}",
        stats.frame_time_ms,
    );
    assert!(stats.frame_time_ms < 100.0, "expected a realistic frame time, got {}", stats.frame_time_ms);
}

#[test]
fn committed_values_stay_stable_until_the_next_window() {
    let mut stats = make_stats();

    let start = Instant::now();
    while start.elapsed().as_secs_f32() < SAMPLE_WINDOW_SECS_DEFAULT + 0.05 {
        stats.tick(1.0 / 60.0);
        sleep(Duration::from_millis(16));
    }

    let first_sample = (stats.fps, stats.frame_time_ms);
    assert!(first_sample.0 > 0.0);

    for _ in 0..5 {
        stats.tick(0.020);
        sleep(Duration::from_millis(5));
    }

    assert_eq!((stats.fps, stats.frame_time_ms), first_sample);
}

#[test]
fn gpu_stats_survive_until_the_time_window_commits() {
    let mut stats = make_stats();

    stats.set_gpu_stats(12, 8_000);
    let start = Instant::now();
    while start.elapsed().as_secs_f32() < SAMPLE_WINDOW_SECS_DEFAULT + 0.05 {
        stats.tick(1.0 / 60.0);
        sleep(Duration::from_millis(16));
    }

    assert_eq!(stats.draw_calls, 12);
    assert_eq!(stats.triangle_count, 8_000);
}

#[test]
fn gpu_stats_update_immediately() {
    let mut stats = make_stats();

    stats.set_gpu_stats(12, 8_000);

    assert_eq!(stats.draw_calls, 12);
    assert_eq!(stats.triangle_count, 8_000);
}

#[test]
fn custom_sample_window_commits_early() {
    let custom_window = 0.1; // Much shorter than the default 0.5s
    let mut stats = make_stats().with_sample_window(custom_window);

    let start = Instant::now();

    // Wait just past the custom 0.1s window
    while start.elapsed().as_secs_f32() < custom_window + 0.05 {
        stats.tick(1.0 / 60.0);
        sleep(Duration::from_millis(16));
    }

    assert!(
        stats.fps > 0.0,
        "expected stats to commit early after {}s, got {}",
        custom_window, stats.fps
    );
}

#[test]
fn custom_sample_window_delays_commit() {
    let custom_window = 0.8; // Longer than the default 0.5s
    let mut stats = make_stats().with_sample_window(custom_window);

    let start = Instant::now();

    // Wait past the default 0.5s window, but BEFORE our custom 0.8s window
    while start.elapsed().as_secs_f32() < SAMPLE_WINDOW_SECS_DEFAULT + 0.1 {
        stats.tick(1.0 / 60.0);
        sleep(Duration::from_millis(16));
    }

    // It should NOT have committed yet, because our window is 0.8s
    assert_eq!(
        stats.fps, 0.0,
        "stats should not commit at the default interval when a longer custom window is set"
    );

    // Now continue waiting until we pass our custom 0.8s window
    while start.elapsed().as_secs_f32() < custom_window + 0.05 {
        stats.tick(1.0 / 60.0);
        sleep(Duration::from_millis(16));
    }

    // Now it should have committed
    assert!(
        stats.fps > 0.0,
        "expected stats to finally commit after {}s, got {}",
        custom_window, stats.fps
    );
}


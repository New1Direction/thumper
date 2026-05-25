use std::time::{Duration, Instant};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum JobStage {
    Resolving,
    Downloading,
    Verifying,
    PackageOp,     // install / add / remove — the 📦 personality
    ScriptRunning, // run / script — the 🚀 personality
    Complete,
}

#[derive(Clone)]
pub struct Job {
    pub id: u64,
    pub command: String,
    pub stage: JobStage,
    pub start_time: Instant,
    pub elapsed: Duration,
    pub progress: f32, // Bound: 0.0 to 1.0
    pub velocity: f32, // Bound: 0.0 to 1.0+ (exponential moving average metric)
    /// High-resolution continuous animation time (seconds since App start).
    /// This enables silky sub-tick sine/smoothstep plasma motion independent of the 120ms tick rate.
    pub animation_time: f64,
    pub completion_time: Option<Instant>,
    pub performance_score: Option<f32>,
    /// Captured stderr / diagnostic lines when the job encountered a failure.
    /// When present, the plasma renders in angry MOCHA_RED breathing mode
    /// and a Diagnostic Error Card is shown beneath the bar.
    pub error_diagnostics: Option<Vec<String>>,
}

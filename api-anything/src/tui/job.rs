use std::time::{Duration, Instant};

#[derive(Clone, Copy, PartialEq, Eq)]
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
    pub progress: f32,     // Bound: 0.0 to 1.0
    pub velocity: f32,     // Bound: 0.0 to 1.0+ (exponential moving average metric)
    pub phase_offset: u64, // Driven by global tick increment for animation synchronization
    pub completion_time: Option<Instant>,
    pub performance_score: Option<f32>,
}

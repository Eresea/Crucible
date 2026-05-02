use std::time::{Duration, Instant};

use thiserror::Error;

pub type ModuleResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[derive(Debug, Clone)]
pub struct EngineConfig {
    pub app_name: String,
    pub fixed_timestep: Duration,
    pub max_frame_delta: Duration,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            app_name: "Crucible".to_string(),
            fixed_timestep: Duration::from_secs_f64(1.0 / 60.0),
            max_frame_delta: Duration::from_millis(250),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FrameClock {
    started_at: Instant,
    last_frame_at: Instant,
    frame_index: u64,
}

impl FrameClock {
    #[must_use]
    pub fn new(now: Instant) -> Self {
        Self {
            started_at: now,
            last_frame_at: now,
            frame_index: 0,
        }
    }

    #[must_use]
    pub fn advance(&mut self, now: Instant, max_delta: Duration) -> FrameContext {
        let mut delta = now.saturating_duration_since(self.last_frame_at);
        if delta > max_delta {
            delta = max_delta;
        }

        self.last_frame_at = now;
        self.frame_index += 1;

        FrameContext {
            frame_index: self.frame_index,
            delta,
            total: now.saturating_duration_since(self.started_at),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FrameContext {
    pub frame_index: u64,
    pub delta: Duration,
    pub total: Duration,
}

impl FrameContext {
    #[must_use]
    pub fn delta_seconds(self) -> f32 {
        self.delta.as_secs_f32()
    }

    #[must_use]
    pub fn total_seconds(self) -> f32 {
        self.total.as_secs_f32()
    }
}

#[derive(Debug, Default)]
pub struct EngineContext {
    shutdown_requested: bool,
}

impl EngineContext {
    pub fn request_shutdown(&mut self) {
        self.shutdown_requested = true;
    }

    #[must_use]
    pub fn is_shutdown_requested(&self) -> bool {
        self.shutdown_requested
    }
}

pub trait EngineModule: Send {
    fn name(&self) -> &'static str;

    fn initialize(&mut self, _context: &mut EngineContext) -> ModuleResult {
        Ok(())
    }

    fn update(&mut self, _context: &mut EngineContext, _frame: FrameContext) -> ModuleResult {
        Ok(())
    }

    fn shutdown(&mut self, _context: &mut EngineContext) -> ModuleResult {
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum EngineError {
    #[error("engine module `{module}` failed during {phase}: {source}")]
    Module {
        module: &'static str,
        phase: &'static str,
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}

pub struct Engine {
    config: EngineConfig,
    context: EngineContext,
    clock: FrameClock,
    modules: Vec<Box<dyn EngineModule>>,
    initialized: bool,
}

impl Engine {
    #[must_use]
    pub fn new(config: EngineConfig) -> Self {
        let now = Instant::now();
        Self {
            config,
            context: EngineContext::default(),
            clock: FrameClock::new(now),
            modules: Vec::new(),
            initialized: false,
        }
    }

    #[must_use]
    pub fn config(&self) -> &EngineConfig {
        &self.config
    }

    #[must_use]
    pub fn context(&self) -> &EngineContext {
        &self.context
    }

    pub fn add_module<M>(&mut self, module: M)
    where
        M: EngineModule + 'static,
    {
        self.modules.push(Box::new(module));
    }

    pub fn initialize(&mut self) -> Result<(), EngineError> {
        if self.initialized {
            return Ok(());
        }

        for module in &mut self.modules {
            module
                .initialize(&mut self.context)
                .map_err(|source| EngineError::Module {
                    module: module.name(),
                    phase: "initialize",
                    source,
                })?;
        }

        self.initialized = true;
        Ok(())
    }

    pub fn tick(&mut self, now: Instant) -> Result<FrameContext, EngineError> {
        let frame = self.clock.advance(now, self.config.max_frame_delta);

        for module in &mut self.modules {
            module
                .update(&mut self.context, frame)
                .map_err(|source| EngineError::Module {
                    module: module.name(),
                    phase: "update",
                    source,
                })?;
        }

        Ok(frame)
    }

    pub fn shutdown(&mut self) -> Result<(), EngineError> {
        for module in self.modules.iter_mut().rev() {
            module
                .shutdown(&mut self.context)
                .map_err(|source| EngineError::Module {
                    module: module.name(),
                    phase: "shutdown",
                    source,
                })?;
        }

        self.initialized = false;
        Ok(())
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new(EngineConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;

    #[derive(Default)]
    struct RecordingModule {
        events: Arc<Mutex<Vec<&'static str>>>,
    }

    impl EngineModule for RecordingModule {
        fn name(&self) -> &'static str {
            "recording"
        }

        fn initialize(&mut self, _context: &mut EngineContext) -> ModuleResult {
            self.events.lock().unwrap().push("initialize");
            Ok(())
        }

        fn update(&mut self, _context: &mut EngineContext, _frame: FrameContext) -> ModuleResult {
            self.events.lock().unwrap().push("update");
            Ok(())
        }

        fn shutdown(&mut self, _context: &mut EngineContext) -> ModuleResult {
            self.events.lock().unwrap().push("shutdown");
            Ok(())
        }
    }

    #[test]
    fn modules_run_in_engine_order_and_shutdown_in_reverse() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut engine = Engine::default();
        engine.add_module(RecordingModule {
            events: Arc::clone(&events),
        });

        engine.initialize().unwrap();
        engine.tick(Instant::now()).unwrap();
        engine.shutdown().unwrap();

        assert_eq!(
            events.lock().unwrap().as_slice(),
            ["initialize", "update", "shutdown"]
        );
    }
}

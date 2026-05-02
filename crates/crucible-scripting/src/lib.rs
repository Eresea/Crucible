use std::collections::HashMap;

use crucible_core::{FrameContext, ModuleResult};
use thiserror::Error;

#[derive(Debug, Clone)]
pub enum ScriptValue {
    Bool(bool),
    Number(f64),
    Text(String),
}

#[derive(Debug, Default)]
pub struct ScriptWorld {
    globals: HashMap<String, ScriptValue>,
}

impl ScriptWorld {
    pub fn set_global(&mut self, name: impl Into<String>, value: ScriptValue) {
        self.globals.insert(name.into(), value);
    }

    #[must_use]
    pub fn global(&self, name: &str) -> Option<&ScriptValue> {
        self.globals.get(name)
    }
}

pub trait ScriptComponent: Send {
    fn name(&self) -> &'static str;

    fn start(&mut self, _world: &mut ScriptWorld) -> ModuleResult {
        Ok(())
    }

    fn update(&mut self, _world: &mut ScriptWorld, _frame: FrameContext) -> ModuleResult {
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum ScriptingError {
    #[error("script `{script}` failed during {phase}: {source}")]
    Script {
        script: &'static str,
        phase: &'static str,
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}

#[derive(Default)]
pub struct NativeScriptHost {
    world: ScriptWorld,
    scripts: Vec<Box<dyn ScriptComponent>>,
    started: bool,
}

impl NativeScriptHost {
    #[must_use]
    pub fn world(&self) -> &ScriptWorld {
        &self.world
    }

    pub fn world_mut(&mut self) -> &mut ScriptWorld {
        &mut self.world
    }

    pub fn add_script<S>(&mut self, script: S)
    where
        S: ScriptComponent + 'static,
    {
        self.scripts.push(Box::new(script));
    }

    pub fn start(&mut self) -> Result<(), ScriptingError> {
        if self.started {
            return Ok(());
        }

        for script in &mut self.scripts {
            script
                .start(&mut self.world)
                .map_err(|source| ScriptingError::Script {
                    script: script.name(),
                    phase: "start",
                    source,
                })?;
        }

        self.started = true;
        Ok(())
    }

    pub fn update(&mut self, frame: FrameContext) -> Result<(), ScriptingError> {
        for script in &mut self.scripts {
            script
                .update(&mut self.world, frame)
                .map_err(|source| ScriptingError::Script {
                    script: script.name(),
                    phase: "update",
                    source,
                })?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crucible_core::FrameClock;
    use std::time::Instant;

    struct CounterScript;

    impl ScriptComponent for CounterScript {
        fn name(&self) -> &'static str {
            "counter"
        }

        fn start(&mut self, world: &mut ScriptWorld) -> ModuleResult {
            world.set_global("ticks", ScriptValue::Number(0.0));
            Ok(())
        }

        fn update(&mut self, world: &mut ScriptWorld, _frame: FrameContext) -> ModuleResult {
            let next = match world.global("ticks") {
                Some(ScriptValue::Number(value)) => value + 1.0,
                _ => 1.0,
            };
            world.set_global("ticks", ScriptValue::Number(next));
            Ok(())
        }
    }

    #[test]
    fn native_scripts_can_update_shared_world_state() {
        let mut host = NativeScriptHost::default();
        host.add_script(CounterScript);

        host.start().unwrap();
        let mut clock = FrameClock::new(Instant::now());
        host.update(clock.advance(Instant::now(), std::time::Duration::from_secs(1)))
            .unwrap();

        assert!(matches!(
            host.world().global("ticks"),
            Some(ScriptValue::Number(1.0))
        ));
    }
}

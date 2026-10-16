use crate::ffi::{ForecastProgress, OrbitPreview};
use rocketcon_sim::ForecastReader;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::runtime::Runtime;
use uuid::Uuid;

pub struct ForecastControl {
    generation: AtomicU64,
}

pub fn create_forecast_control() -> Box<ForecastControl> {
    Box::new(ForecastControl {
        generation: AtomicU64::new(0),
    })
}

impl ForecastControl {
    pub fn select_generation(&self, generation: u64) {
        self.generation.store(generation, Ordering::Release);
    }

    pub fn is_cancelled(&self, generation: u64) -> bool {
        self.generation.load(Ordering::Acquire) != generation
    }
}

pub struct PreviewEngine {
    runtime: Runtime,
    reader: Option<ForecastReader>,
}

pub fn create_preview_engine() -> Result<Box<PreviewEngine>, String> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| error.to_string())?;
    Ok(Box::new(PreviewEngine {
        runtime,
        reader: None,
    }))
}

impl PreviewEngine {
    pub fn load_read_only(&mut self, save_path: &str, vehicle_uuid: &str) -> Result<(), String> {
        let id = vehicle_uuid
            .parse::<Uuid>()
            .map_err(|error| error.to_string())?;
        let next = self
            .runtime
            .block_on(ForecastReader::open(save_path, id))
            .map_err(|error| error.to_string())?;
        if let Some(previous) = self.reader.replace(next) {
            self.runtime.block_on(previous.close());
        }
        Ok(())
    }

    pub fn predict(
        &self,
        under_thrust: bool,
        control: &ForecastControl,
        generation: u64,
        mut progress: Pin<&mut ForecastProgress>,
    ) -> Result<OrbitPreview, String> {
        let mut cancelled = || control.is_cancelled(generation);
        if cancelled() {
            return Err("forecast cancelled".into());
        }
        self.runtime
            .block_on(
                self.reader
                    .as_ref()
                    .ok_or("no forecast save loaded")?
                    .preview_cancelled(under_thrust, &mut cancelled, &mut |partial| {
                        if !control.is_cancelled(generation) {
                            progress.as_mut().publish(partial.into(), generation);
                        }
                    }),
            )
            .map(OrbitPreview::from)
            .map_err(|error| error.to_string())
    }
}

impl Drop for PreviewEngine {
    fn drop(&mut self) {
        if let Some(reader) = self.reader.take() {
            self.runtime.block_on(reader.close());
        }
    }
}
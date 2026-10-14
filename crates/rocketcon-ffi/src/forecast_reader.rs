use crate::ffi::OrbitPreview;
use rocketcon_sim::ForecastReader;
use tokio::runtime::Runtime;
use uuid::Uuid;

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

    pub fn predict(&self, under_thrust: bool) -> Result<OrbitPreview, String> {
        self.runtime
            .block_on(
                self.reader
                    .as_ref()
                    .ok_or("no forecast save loaded")?
                    .preview(under_thrust),
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

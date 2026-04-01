use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use crate::model::types::{ProblemModel, SolvePredictionModel};

pub fn save_model(model: &SolvePredictionModel, path: &Path) -> Result<()> {
    let encoded = bincode::serialize(model).context("Failed to serialize model")?;
    let file = File::create(path)
        .with_context(|| format!("Failed to create model file at {}", path.display()))?;
    let mut encoder = GzEncoder::new(file, Compression::default());
    encoder
        .write_all(&encoded)
        .context("Failed to write compressed model")?;
    encoder.finish().context("Failed to finalize gzip stream")?;
    Ok(())
}

pub fn save_problem_model(model: &ProblemModel, path: &Path) -> Result<()> {
    let encoded = bincode::serialize(model).context("Failed to serialize problem model")?;
    let file = File::create(path)
        .with_context(|| format!("Failed to create problem model file at {}", path.display()))?;
    let mut encoder = GzEncoder::new(file, Compression::default());
    encoder
        .write_all(&encoded)
        .context("Failed to write compressed problem model")?;
    encoder
        .finish()
        .context("Failed to finalize gzip stream")?;
    Ok(())
}

pub fn load_problem_model(path: &Path) -> Result<ProblemModel> {
    let file = File::open(path)
        .with_context(|| format!("Failed to open problem model file at {}", path.display()))?;
    let mut decoder = GzDecoder::new(file);
    let mut buf = Vec::new();
    decoder
        .read_to_end(&mut buf)
        .context("Failed to decompress problem model")?;
    let model: ProblemModel =
        bincode::deserialize(&buf).context("Failed to deserialize problem model")?;
    Ok(model)
}

pub fn load_model(path: &Path) -> Result<SolvePredictionModel> {
    let file = File::open(path)
        .with_context(|| format!("Failed to open model file at {}", path.display()))?;
    let mut decoder = GzDecoder::new(file);
    let mut buf = Vec::new();
    decoder
        .read_to_end(&mut buf)
        .context("Failed to decompress model")?;
    let model: SolvePredictionModel =
        bincode::deserialize(&buf).context("Failed to deserialize model")?;
    Ok(model)
}

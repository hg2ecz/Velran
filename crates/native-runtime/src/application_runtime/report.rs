use native_build::incremental_build::BuildDisposition;

#[derive(Debug)]
pub struct RefreshReport {
    pub build_disposition: BuildDisposition,
    pub generation: u64,
    pub activated: bool,
    pub dirty_shards: Vec<String>,
}

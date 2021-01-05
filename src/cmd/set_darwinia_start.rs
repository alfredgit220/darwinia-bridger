use crate::error::Result;
use crate::tools;
use crate::Config;
use std::path::PathBuf;

/// set ethereum start
pub async fn exec(data_dir: Option<PathBuf>, start: u64) -> Result<()> {
	std::env::set_var("RUST_LOG", "info,darwinia_bridger");
	env_logger::init();

	let data_dir = data_dir.unwrap_or(Config::default_data_dir()?);

	tools::set_cache(
		data_dir,
		tools::LAST_TRACKED_ETHEREUM_BLOCK_FILE_NAME,
		start,
	)
	.await?;
	println!("OK");

	Ok(())
}
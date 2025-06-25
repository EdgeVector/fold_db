use crate::logging::config::ConfigError;

/// Parse file size string (e.g., "10MB", "1GB") to bytes
pub fn parse_file_size(size_str: &str) -> Result<u64, ConfigError> {
    let size_str = size_str.to_uppercase();

    if let Some(num_str) = size_str.strip_suffix("GB") {
        let num: u64 = num_str
            .parse()
            .map_err(|_| ConfigError::InvalidFileSize(size_str.clone()))?;
        Ok(num * 1024 * 1024 * 1024)
    } else if let Some(num_str) = size_str.strip_suffix("MB") {
        let num: u64 = num_str
            .parse()
            .map_err(|_| ConfigError::InvalidFileSize(size_str.clone()))?;
        Ok(num * 1024 * 1024)
    } else if let Some(num_str) = size_str.strip_suffix("KB") {
        let num: u64 = num_str
            .parse()
            .map_err(|_| ConfigError::InvalidFileSize(size_str.clone()))?;
        Ok(num * 1024)
    } else if let Some(num_str) = size_str.strip_suffix("B") {
        num_str
            .parse()
            .map_err(|_| ConfigError::InvalidFileSize(size_str.clone()))
    } else {
        // Default to bytes if no suffix
        size_str
            .parse()
            .map_err(|_| ConfigError::InvalidFileSize(size_str.clone()))
    }
}

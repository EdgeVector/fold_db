use datafold::logging::utils::parse_file_size;

#[test]
fn parse_file_size_kilobytes() {
    assert_eq!(parse_file_size("10KB").unwrap(), 10 * 1024);
}

#[test]
fn parse_file_size_megabytes() {
    assert_eq!(parse_file_size("2MB").unwrap(), 2 * 1024 * 1024);
}

#[test]
fn parse_file_size_gigabytes() {
    assert_eq!(parse_file_size("1GB").unwrap(), 1024 * 1024 * 1024);
}

#[test]
fn parse_file_size_no_suffix() {
    assert_eq!(parse_file_size("500").unwrap(), 500);
}

#[test]
fn parse_file_size_invalid() {
    assert!(parse_file_size("abc").is_err());
}

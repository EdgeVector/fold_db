fn main() {
    let test_str = "String\n";
    let has_control = test_str.chars().any(|c| c.is_control() || c == '\n' || c == '\r');
    println!("String '{}' has control chars: {}", test_str.escape_debug(), has_control);
    
    let newline_char = '\n';
    println!("Newline is_control(): {}", newline_char.is_control());
}

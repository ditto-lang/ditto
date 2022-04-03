pub static INDENT_WIDTH: u8 = 4;
pub static MAX_WIDTH: u32 = 80;

#[cfg(windows)]
pub static NEWLINE: &str = "\r\n";

#[cfg(not(windows))]
pub static NEWLINE: &str = "\n";

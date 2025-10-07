/// Capitalizes the first character in s.
/// Shamelessly stolen from [here](https://nick.groenen.me/notes/capitalize-a-string-in-rust/)
/// which attributes it to [here](https://stackoverflow.com/a/38406885)
///
/// Why tf is this not in std?
pub fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

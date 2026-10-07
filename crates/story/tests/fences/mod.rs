//! The parts of a prompt outside its fences, where no game name may stand (GAMEPLAY.md
//! 3.2.1).

/// The lines of a prompt that stand outside every fence and hold `name`.
pub fn unfenced_lines_with<'a>(prompt: &'a str, name: &str) -> Vec<&'a str> {
    let mut inside = false;
    let mut found = Vec::new();
    for line in prompt.lines() {
        match line {
            "<<<" => inside = true,
            ">>>" => inside = false,
            _ if !inside && line.contains(name) => found.push(line),
            _ => {}
        }
    }
    found
}

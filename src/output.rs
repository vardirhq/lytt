use crate::PortEntry;

pub fn print_table(entries: &[PortEntry]) {
    if entries.is_empty() {
        println!("No listening TCP ports found.");
        return;
    }

    println!(
        "{:<7} {:<7} {:<8} {:<18} {:<12} URL",
        "PORT", "PROTO", "PID", "COMMAND", "PROJECT"
    );

    for entry in entries {
        let pid = entry
            .process
            .as_ref()
            .map(|process| process.pid.to_string())
            .unwrap_or_else(|| "-".to_string());
        let command = entry
            .process
            .as_ref()
            .map(|process| truncate(process.display_name(), 18))
            .unwrap_or_else(|| "-".to_string());
        let project = entry
            .process
            .as_ref()
            .and_then(|process| process.project.as_ref())
            .map(|project| truncate(&project.name, 12))
            .unwrap_or_else(|| "-".to_string());

        println!(
            "{:<7} {:<7} {:<8} {:<18} {:<12} {}",
            entry.port,
            entry.protocol,
            pid,
            command,
            project,
            url_for(entry)
        );
    }
}

pub fn print_json(entries: &[PortEntry]) {
    println!("[");
    for (index, entry) in entries.iter().enumerate() {
        let comma = if index + 1 == entries.len() { "" } else { "," };
        println!(
            "  {{\"port\":{},\"protocol\":\"{}\",\"address\":\"{}\",\"inode\":\"{}\",\"url\":\"{}\",\"process\":{}}}{}",
            entry.port,
            escape_json(&entry.protocol),
            escape_json(&entry.address),
            escape_json(&entry.inode),
            escape_json(&url_for(entry)),
            process_json(entry),
            comma
        );
    }
    println!("]");
}

fn process_json(entry: &PortEntry) -> String {
    let Some(process) = &entry.process else {
        return "null".to_string();
    };

    let cwd = process
        .cwd
        .as_ref()
        .map(|path| format!("\"{}\"", escape_json(&path.display().to_string())))
        .unwrap_or_else(|| "null".to_string());
    let project = process
        .project
        .as_ref()
        .map(|project| {
            format!(
                "{{\"name\":\"{}\",\"kind\":\"{}\",\"root\":\"{}\"}}",
                escape_json(&project.name),
                escape_json(&project.kind),
                escape_json(&project.root.display().to_string())
            )
        })
        .unwrap_or_else(|| "null".to_string());

    format!(
        "{{\"pid\":{},\"command\":\"{}\",\"cwd\":{},\"project\":{}}}",
        process.pid,
        escape_json(&process.command),
        cwd,
        project
    )
}

fn url_for(entry: &PortEntry) -> String {
    let host = if entry.address == "0.0.0.0" || entry.address == "::" {
        "localhost".to_string()
    } else {
        entry.address.clone()
    };

    format!("http://{host}:{}", entry.port)
}

fn truncate(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        return value.to_string();
    }

    if max <= 3 {
        return ".".repeat(max);
    }

    let mut shortened = value.chars().take(max - 3).collect::<String>();
    shortened.push_str("...");
    shortened
}

fn escape_json(value: &str) -> String {
    value
        .chars()
        .flat_map(|character| match character {
            '"' => "\\\"".chars().collect::<Vec<_>>(),
            '\\' => "\\\\".chars().collect::<Vec<_>>(),
            '\n' => "\\n".chars().collect::<Vec<_>>(),
            '\r' => "\\r".chars().collect::<Vec<_>>(),
            '\t' => "\\t".chars().collect::<Vec<_>>(),
            other => vec![other],
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_json_strings() {
        assert_eq!(escape_json("a\"b\\c"), "a\\\"b\\\\c");
    }

    #[test]
    fn truncates_long_values() {
        assert_eq!(truncate("localhost", 5), "lo...");
    }
}

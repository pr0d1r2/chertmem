use super::{date_of, output, this};

#[test]
fn this_machine_has_a_cpu_model_and_an_os() -> Result<(), String> {
    let host = this().ok_or("no host")?;
    assert!(!host.machine.is_empty());
    let name = crate::baseline::os_name(&host.os);
    assert!(["macOS", "Linux"].contains(&name), "{}", host.os);
    Ok(())
}

#[test]
fn a_file_is_dated_by_its_write() -> Result<(), String> {
    let date = date_of(std::path::Path::new("Cargo.toml")).ok_or("no date")?;
    let shape = |c: char| c.is_ascii_digit() || c == '-';
    assert!(date.len() == 10 && date.chars().all(shape), "{date}");
    assert_eq!(date_of(std::path::Path::new("no/such/file")), None);
    Ok(())
}

#[test]
fn a_failing_or_silent_command_is_none() {
    assert_eq!(output("false", &[]), None);
    assert_eq!(output("true", &[]), None);
    assert_eq!(output("no-such-command-here", &[]), None);
}

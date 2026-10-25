use super::{date_of, model, output, this};

const X86: &str = "processor\t: 0\nvendor_id\t: GenuineIntel\nmodel name\t: Intel(R) Xeon(R) Platinum 8370C CPU @ 2.80GHz\n";
const ARM: &str = "processor\t: 0\nBogoMIPS\t: 50.00\nCPU implementer\t: 0x41\nCPU architecture: 8\nCPU variant\t: 0x3\nCPU part\t: 0xd0c\nCPU revision\t: 1\n";

#[test]
fn an_x86_cpuinfo_names_its_model_name() {
    let named = model(X86);
    assert_eq!(
        named.as_deref(),
        Some("Intel(R) Xeon(R) Platinum 8370C CPU @ 2.80GHz")
    );
}

#[test]
fn an_arm_cpuinfo_without_a_model_name_names_implementer_and_part() {
    let named = model(ARM);
    assert_eq!(named.as_deref(), Some("ARM implementer 0x41 part 0xd0c"));
}

#[test]
fn a_cpuinfo_naming_no_cpu_is_none() {
    assert_eq!(model("processor\t: 0\n"), None);
}

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

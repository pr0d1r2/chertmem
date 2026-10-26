use super::{date_of, model, named, output, this};

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

/// `big.LITTLE`: each core prints its own implementer and part, so 2 hosts
/// sharing their 1st core but not the rest must get 2 names.
#[test]
fn a_mixed_core_arm_cpuinfo_names_every_distinct_core_in_order() {
    let block = |part: &str| {
        format!(
            "processor\t: 0\nCPU implementer\t: 0x41\nCPU part\t: {part}\n\n"
        )
    };
    let mixed = [block("0xd05"), block("0xd05"), block("0xd0b")].concat();
    let named = model(&mixed);
    let want = "ARM implementer 0x41 part 0xd05 + implementer 0x41 part 0xd0b";
    assert_eq!(named.as_deref(), Some(want));
    assert_ne!(model(&block("0xd05")), named);
}

/// Review on PR #2: an unreadable `/proc/cpuinfo` still names the
/// machine through `uname -m`, not `None`.
#[test]
fn an_unreadable_cpuinfo_falls_back_to_the_machine_arch() {
    let arch = output("uname", &["-m"]);
    assert!(arch.is_some());
    assert_eq!(named(None), arch);
    assert_eq!(named(Some("processor\t: 0\n")), arch);
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

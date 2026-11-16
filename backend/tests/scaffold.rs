#[test]
fn executable_reports_its_version_without_external_services() {
    let result = std::process::Command::new(env!("CARGO_BIN_EXE_meetagain"))
        .output()
        .expect("executable starts");
    assert!(result.status.success());
    assert!(
        String::from_utf8(result.stdout)
            .unwrap()
            .contains(env!("CARGO_PKG_VERSION"))
    );
}

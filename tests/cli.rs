// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-log-converter — https://github.com/bboxy450/bboxy450-log-converter
// License: https://github.com/bboxy450/bboxy450-log-converter/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-log-converter/blob/main/ATTRIBUTION.md

use std::process::Command;
#[test]
fn help_version_and_skeleton_status_are_honest() {
    let cli = env!("CARGO_BIN_EXE_bboxy450-log-converter");
    let help = Command::new(cli).arg("--help").output().unwrap();
    assert!(help.status.success());
    let text = String::from_utf8(help.stdout).unwrap();
    assert!(text.contains("Bboxy"));
    assert!(!text.contains("bboxy450-harness"));
    for command in [
        "parse",
        "lower",
        "convert",
        "sanitize",
        "cat",
        "watch",
        "switch-prepare",
        "export",
    ] {
        assert!(text.contains(command));
        assert_eq!(
            Command::new(cli)
                .arg(command)
                .output()
                .unwrap()
                .status
                .code(),
            Some(2)
        );
    }
    assert!(
        Command::new(cli)
            .arg("--version")
            .output()
            .unwrap()
            .status
            .success()
    );
    assert_eq!(
        Command::new(cli)
            .arg("bogus")
            .output()
            .unwrap()
            .status
            .code(),
        Some(2)
    );
}

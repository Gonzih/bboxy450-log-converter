// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-log-converter — https://github.com/bboxy450/bboxy450-log-converter
// License: https://github.com/bboxy450/bboxy450-log-converter/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-log-converter/blob/main/ATTRIBUTION.md

use bboxy450_log_converter::{ir::SessionId, lower::codex, parse::codex::Parser};
use std::{fs, io::Cursor, path::PathBuf, process::Command};
#[allow(dead_code)]
#[path = "support/codex_fixture.rs"]
mod fixture;
struct Temp(PathBuf);
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
#[test]
fn lowered_codex_resumes_in_real_cli_with_prior_turns() {
    let dir = Temp(std::env::temp_dir().join(format!("lc-p4-native-{}", ulid::Ulid::new())));
    fs::create_dir(&dir.0).unwrap();
    let input = fixture::resumable(dir.0.to_str().unwrap());
    let mut parser = Parser::new(Cursor::new(&input), SessionId(ulid::Ulid::from(4u128))).unwrap();
    let mut events = vec![];
    while let Some(batch) = parser.next_record().unwrap() {
        events.extend(batch);
    }
    let mut lowered = vec![];
    codex::write(parser.session(), &events, &mut lowered).unwrap();
    assert_eq!(input, lowered);
    let path = dir.0.join("lowered.jsonl");
    fs::write(&path, lowered).unwrap();
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let output = Command::new("python3")
        .arg("-I")
        .arg(manifest.join("scripts/test-codex-resume.py"))
        .arg(&path)
        .arg(manifest.join(".bboxy/runs/p4-resume-evidence.log"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    println!("{}", String::from_utf8_lossy(&output.stdout));
}

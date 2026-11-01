// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-log-converter — https://github.com/bboxy450/bboxy450-log-converter
// License: https://github.com/bboxy450/bboxy450-log-converter/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-log-converter/blob/main/ATTRIBUTION.md

//! CLI surface for the staged converter implementation.
#![forbid(unsafe_code)]
use std::process::ExitCode;

const HELP: &str = "bboxy450-log-converter — Claude Code, Codex and Bboxy session logs

Usage: bboxy450-log-converter <COMMAND>

Commands (conversion implementation follows in later phases):
  parse <provider> <file> -o <bxl.d>
  lower <bxl.d> --to <provider> [-o <file>]
  convert <file> --to <provider>
  sanitize <bxl.d>
  cat <bxl.d>
  watch <provider> <file> --sink file:<dir>
  switch-prepare
  export atif <bxl.d>

Providers: claude, codex, bboxy
Options: -h, --help; --version
";

fn main() -> ExitCode {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.is_empty() || args.iter().any(|a| a == "--help" || a == "-h") {
        print!("{HELP}");
        return ExitCode::SUCCESS;
    }
    if args == ["--version"] {
        println!("bboxy450-log-converter {}", env!("CARGO_PKG_VERSION"));
        return ExitCode::SUCCESS;
    }
    if matches!(
        args[0].as_str(),
        "parse" | "lower" | "convert" | "sanitize" | "cat" | "watch" | "switch-prepare" | "export"
    ) {
        eprintln!(
            "{}: command reserved; implementation is not available in this bootstrap",
            args[0]
        );
    } else {
        eprintln!("unknown command: {} (use --help)", args[0]);
    }
    ExitCode::from(2)
}

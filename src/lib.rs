// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-log-converter — https://github.com/bboxy450/bboxy450-log-converter
// License: https://github.com/bboxy450/bboxy450-log-converter/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-log-converter/blob/main/ATTRIBUTION.md

//! Common, loss-aware IR for Claude Code, Codex and Bboxy session logs.
//! Native codecs retain source records; the bxl transport lands in later phases.
#![forbid(unsafe_code)]

pub mod ir;

pub mod lower;
pub mod parse;

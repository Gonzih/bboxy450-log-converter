// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-log-converter — https://github.com/bboxy450/bboxy450-log-converter
// License: https://github.com/bboxy450/bboxy450-log-converter/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-log-converter/blob/main/ATTRIBUTION.md

// Adapted from https://github.com/bboxy450/bboxy450-harness/blob/main/tests/formal/export.rs
// Upstream origin notices retained below.
// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-harness — https://github.com/bboxy450/bboxy450-harness
// License: https://github.com/bboxy450/bboxy450-harness/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-harness/blob/main/ATTRIBUTION.md
// Adapted from bboxy450-term: https://github.com/bboxy450/bboxy450-term/blob/main/crates/bboxy450-tlc/examples/verify.rs
use bboxy450_tlc::Model;
use std::{collections::BTreeSet, fmt::Write as _, path::Path};
pub fn export(
    m: Model,
    expected: &str,
    root: &Path,
    manifest: &mut String,
) -> Result<(), Box<dyn std::error::Error>> {
    let metadata = serde_json::json!({
        "variables": m.variables.iter().map(|v| serde_json::json!({"name":v.name,"min":v.min,"max":v.max,"initial":v.initial})).collect::<Vec<_>>(),
        "actions": m.actions.iter().map(|a| &a.name).collect::<Vec<_>>(),
        "invariants": m.invariants.iter().map(|v| &v.name).collect::<Vec<_>>(),
        "responses": m.responses.iter().map(|v| &v.name).collect::<Vec<_>>(),
        "weak_fair": m.weak_fair,
        "implementation_refinement_proven": false
    });
    std::fs::write(
        root.join(format!("{}.metadata.json", m.name)),
        serde_json::to_vec_pretty(&metadata)?,
    )?;
    let r = m.check(100_000)?;
    let actual = if r.violations.is_empty() {
        "pass"
    } else if r.violations.iter().any(|v| !v.cycle.is_empty()) {
        "liveness"
    } else if r.violations.iter().any(|v| v.property == "Deadlock") {
        "deadlock"
    } else {
        "safety"
    };
    if actual != expected {
        return Err(format!("{}: native expected {expected}, got {actual}", m.name).into());
    }
    std::fs::write(root.join(format!("{}.tla", m.name)), m.tla()?)?;
    std::fs::write(root.join(format!("{}.cfg", m.name)), m.config(false))?;
    std::fs::write(root.join(format!("{}Graph.cfg", m.name)), m.config(true))?;
    let states: BTreeSet<_> = r.states.iter().cloned().collect();
    let mut rows = String::new();
    for state in states {
        writeln!(
            &mut rows,
            "{}",
            state
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(",")
        )?;
    }
    std::fs::write(root.join(format!("{}.states", m.name)), rows)?;
    let mut transitions = BTreeSet::new();
    for (i, edges) in r.edges.iter().enumerate() {
        for edge in edges {
            transitions.insert(format!(
                "{:?}\t{}\t{:?}",
                r.states[i], edge.action, r.states[edge.to]
            ));
        }
    }
    std::fs::write(
        root.join(format!("{}.edges", m.name)),
        transitions.into_iter().collect::<Vec<_>>().join("\n") + "\n",
    )?;
    let mut trace = String::new();
    for v in &r.violations {
        writeln!(
            &mut trace,
            "{} prefix={:?} cycle={:?}",
            v.property,
            v.prefix.iter().map(|i| &r.states[*i]).collect::<Vec<_>>(),
            v.cycle.iter().map(|i| &r.states[*i]).collect::<Vec<_>>()
        )?;
    }
    std::fs::write(root.join(format!("{}.native-trace", m.name)), trace)?;
    writeln!(
        manifest,
        "{}\t{}\t{}\t{}",
        m.name,
        expected,
        r.states.len(),
        m.variables.len()
    )?;
    Ok(())
}

// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-log-converter — https://github.com/bboxy450/bboxy450-log-converter
// License: https://github.com/bboxy450/bboxy450-log-converter/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-log-converter/blob/main/ATTRIBUTION.md

//! Checker plumbing only. No product stateful contracts exist in P1.
use bboxy450_tlc::{
    Action,
    Expr::{Bool as B, Int as I, Var as V},
    Invariant, Model, Response, Variable,
};

/// Later phases register product contracts and controls here and in both
/// independent inventories. A model added in only one place fails the gate.
pub fn product_contracts() -> Vec<(Model, &'static str)> {
    vec![]
}

pub fn corpus() -> Vec<(Model, &'static str)> {
    let mut counter = Model::new("GateCounter", vec![Variable::new("count", 0, 2, 0)]);
    counter.actions.push(Action::new(
        "increment",
        V(0).lt(I(2)),
        vec![(0, V(0).plus(I(1)))],
    ));
    counter.check_deadlock = false; // count=2 is a deliberate terminal state.
    counter.invariants.push(Invariant {
        name: "Bound".into(),
        condition: V(0).le(I(2)),
    });
    let mut safety = counter.clone();
    safety.name = "BrokenGateSafety".into();
    safety.invariants[0].condition = V(0).le(I(1));
    let mut deadlock = counter.clone();
    deadlock.name = "BrokenGateDeadlock".into();
    deadlock.check_deadlock = true;
    let mut fair = Model::new(
        "GateFairResponse",
        vec![
            Variable::new("done", 0, 1, 0),
            Variable::new("tick", 0, 1, 0),
        ],
    );
    fair.actions
        .push(Action::new("serve", V(0).eq(I(0)), vec![(0, I(1))]));
    fair.actions
        .push(Action::new("tick", B(true), vec![(1, I(1).minus(V(1)))]));
    fair.weak_fair.push(vec![0]);
    fair.responses.push(Response {
        name: "EventuallyServed".into(),
        request: B(true),
        response: V(0).eq(I(1)),
    });
    let mut unfair = fair.clone();
    unfair.name = "BrokenGateLiveness".into();
    unfair.weak_fair.clear();
    let mut cases = vec![
        (counter, "pass"),
        (safety, "safety"),
        (deadlock, "deadlock"),
        (fair, "pass"),
        (unfair, "liveness"),
    ];
    cases.extend(product_contracts());
    cases
}

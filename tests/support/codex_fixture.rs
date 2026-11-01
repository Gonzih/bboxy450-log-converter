// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-log-converter — https://github.com/bboxy450/bboxy450-log-converter
// License: https://github.com/bboxy450/bboxy450-log-converter/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-log-converter/blob/main/ATTRIBUTION.md

//! Hand-written synthetic records; never sourced from a native user session.
use serde_json::{Value, json};
pub const SID: &str = "00000000-0000-4000-8000-000000000004";
pub const FIRST: &str = "LC_SYNTHETIC_TURN_ONE_7e943b";
pub const SECOND: &str = "LC_SYNTHETIC_TURN_TWO_12aa09";
fn line(kind: &str, payload: Value) -> Value {
    json!({"timestamp":"2026-11-01T12:00:00.000Z","type":kind,"payload":payload})
}
pub fn header(cwd: &str) -> Value {
    line(
        "session_meta",
        json!({"id":SID,"session_id":SID,"timestamp":"2026-11-01T12:00:00.000Z",
        "cwd":cwd,"cli_version":"0.164.0","source":"cli","originator":"lc_synthetic",
        "model_provider":"lc_mock","base_instructions":{"text":"Synthetic test instructions."},
        "git":{"branch":"synthetic","commit_hash":"abc","repository_url":"https://example.test/synthetic"}}),
    )
}
fn message(role: &str, text: &str) -> Value {
    line(
        "response_item",
        json!({"type":"message","role":role,"content":[{"type":if role=="assistant" {"output_text"} else {"input_text"},"text":text}]}),
    )
}
pub fn resumable(cwd: &str) -> Vec<u8> {
    encode(vec![
        header(cwd),
        message("user", FIRST),
        message("assistant", "LC_ASSISTANT_ONE"),
        message("user", SECOND),
        message("assistant", "LC_ASSISTANT_TWO"),
    ])
}
pub fn comprehensive() -> Vec<u8> {
    encode(vec![
        header("/synthetic"),
        line(
            "event_msg",
            json!({"type":"task_started","turn_id":"turn-one"}),
        ),
        line(
            "turn_context",
            json!({"cwd":"/synthetic","model":"synthetic","approval_policy":"never"}),
        ),
        message("user", "hello 🦊"),
        message("developer", "synthetic developer"),
        message("assistant", "synthetic reply"),
        line(
            "response_item",
            json!({"type":"message","role":"user","content":[{"type":"input_image","image_url":"data:image/png;base64,YWJj"},{"type":"input_image","image_url":"https://example.test/not-fetched"}]}),
        ),
        line(
            "response_item",
            json!({"type":"reasoning","id":"rs_synthetic","summary":[{"type":"summary_text","text":"readable summary"}],"content":[{"type":"reasoning_text","text":"readable reasoning"}],"encrypted_content":"LC_SYNTHETIC_SEALED_REASONING"}),
        ),
        line(
            "response_item",
            json!({"type":"function_call","call_id":"call_fn","namespace":"functions","name":"exec_command","arguments":"{\"cmd\":\"synthetic\"}"}),
        ),
        line(
            "response_item",
            json!({"type":"function_call_output","call_id":"call_fn","output":[{"type":"output_text","text":"synthetic function output"}]}),
        ),
        line(
            "response_item",
            json!({"type":"custom_tool_call","call_id":"call_custom","name":"exec","input":"synthetic JavaScript text"}),
        ),
        line(
            "response_item",
            json!({"type":"custom_tool_call_output","call_id":"call_custom","is_error":true,"output":"synthetic custom failure"}),
        ),
        line(
            "event_msg",
            json!({"type":"token_count","info":{"last_token_usage":{"input_tokens":11,"output_tokens":12,"cached_input_tokens":13,"cache_write_input_tokens":14,"reasoning_output_tokens":15}}}),
        ),
        line("world_state", json!({"synthetic":true})),
        line("token_usage_record", json!({"synthetic":true})),
        line(
            "inter_agent_communication_metadata",
            json!({"synthetic":true}),
        ),
        line("future_record", json!({"preserve":"unknown"})),
        line(
            "response_item",
            json!({"type":"future_item","preserve":true}),
        ),
        line(
            "event_msg",
            json!({"type":"user_message","message":"synthetic UI input"}),
        ),
        line(
            "event_msg",
            json!({"type":"agent_message","message":"synthetic UI reply"}),
        ),
        line(
            "event_msg",
            json!({"type":"agent_reasoning","text":"synthetic UI reasoning"}),
        ),
        line(
            "event_msg",
            json!({"type":"warning","message":"synthetic warning"}),
        ),
        line(
            "event_msg",
            json!({"type":"error","message":"synthetic error"}),
        ),
        line(
            "compacted",
            json!({"message":"synthetic summary","replacement_history":[
            {"type":"message","role":"developer","content":[{"type":"input_text","text":"replacement context"}]},
            {"type":"compaction","encrypted_content":"LC_SYNTHETIC_SEALED_COMPACTION"}]}),
        ),
        line(
            "response_item",
            json!({"type":"compaction","encrypted_content":"LC_SYNTHETIC_SEALED_ITEM"}),
        ),
        line("event_msg", json!({"type":"task_complete"})),
        line(
            "event_msg",
            json!({"type":"turn_started","turn_id":"turn-two"}),
        ),
        line("event_msg", json!({"type":"turn_aborted"})),
    ])
}
pub fn encode(rows: Vec<Value>) -> Vec<u8> {
    let mut bytes = vec![];
    for (ordinal, mut row) in rows.into_iter().enumerate() {
        row["ordinal"] = json!(ordinal);
        bytes.extend_from_slice(
            format!("  {} \r\n", serde_json::to_string(&row).unwrap()).as_bytes(),
        );
    }
    bytes
}

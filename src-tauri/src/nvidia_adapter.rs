//! Built-in Anthropic Messages <-> NVIDIA Chat Completions adapter.
//! No external process/dependency, credentials, prompts or responses are logged.
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::TcpStream;
const LIMIT: usize = 16 * 1024 * 1024;
fn invalid() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        "invalid or unsupported NVIDIA response",
    )
}
fn text(v: &Value) -> Result<String, &'static str> {
    if let Some(s) = v.as_str() {
        return Ok(s.to_owned());
    }
    let blocks = v.as_array().ok_or("expected text content")?;
    let mut out = String::new();
    for b in blocks {
        if b["type"] != "text" {
            return Err("this NVIDIA connection supports text tool results only");
        }
        if !out.is_empty() {
            out.push_str("\n\n");
        }
        out.push_str(b["text"].as_str().ok_or("text block is missing text")?);
    }
    Ok(out)
}
pub fn request(body: &[u8], model: Option<&str>) -> Result<Value, &'static str> {
    let v: Value = serde_json::from_slice(body).map_err(|_| "invalid Messages JSON")?;
    let model = model
        .or_else(|| v["model"].as_str())
        .filter(|s| !s.is_empty())
        .ok_or("choose a NVIDIA model")?;
    if v.get("output_config")
        .and_then(|c| c.get("format"))
        .is_some()
    {
        return Err("NVIDIA structured output is not supported by this adapter");
    }
    let mut messages = Vec::new();
    let mut system_parts = Vec::new();
    if let Some(system) = v.get("system") {
        system_parts.push(text(system)?);
    }
    let input = v["messages"]
        .as_array()
        .ok_or("messages must be an array")?;
    for (index, m) in input.iter().enumerate() {
        let role = m["role"].as_str().ok_or("missing message role")?;
        if role == "system" {
            // Claude supports inline system text. NVIDIA accepts one initial
            // system message, so collect active instructions in original order.
            // Cache positions/signatures are Anthropic-specific, not replayed.
            if m.get("output_config").is_some() {
                return Err("NVIDIA does not support per-message effort changes");
            }
            let instruction = text(&m["content"]).map_err(
                |_| "NVIDIA supports text system messages; dynamic tool changes are unavailable",
            )?;
            let active = match m.get("clear_at") {
                None => true,
                Some(Value::String(scope)) if scope == "never" => true,
                Some(Value::String(scope)) if scope == "next_user_message" => !input[index + 1..]
                    .iter()
                    .any(|later| later["role"] == "user"),
                _ => return Err("unsupported system message lifetime"),
            };
            if active {
                system_parts.push(instruction);
            }
            continue;
        }
        if role != "user" && role != "assistant" {
            return Err(
                "unsupported message role; NVIDIA Messages input accepts user, assistant and text system messages",
            );
        }
        if m.get("clear_at").is_some() {
            return Err("message lifetime is only valid for system messages");
        }
        if let Some(s) = m["content"].as_str() {
            messages.push(json!({"role":role,"content":s}));
            continue;
        }
        let mut content = Vec::new();
        let mut calls = Vec::new();
        let mut results = Vec::new();
        for b in m["content"].as_array().ok_or("invalid message content")? {
            match b["type"].as_str() {
                Some("text") => content.push(json!({"type":"text","text":b["text"].as_str().ok_or("missing text")?})),
                Some("image") if role == "user" => {
                    let source = &b["source"];
                    let url = match source["type"].as_str() {
                        Some("base64") => format!("data:{};base64,{}", source["media_type"].as_str().ok_or("missing image MIME type")?,source["data"].as_str().ok_or("missing image data")?),
                        Some("url") => source["url"].as_str().ok_or("missing image URL")?.to_string(),
                        _ => return Err("unsupported image source"),
                    };
                    content.push(json!({"type":"image_url","image_url":{"url":url}}));
                }
                Some("tool_use") if role == "assistant" => calls.push(json!({"id":b["id"].as_str().ok_or("missing tool ID")?,"type":"function","function":{"name":b["name"].as_str().ok_or("missing tool name")?,"arguments":b["input"].to_string()}})),
                Some("tool_result") if role == "user" => results.push(json!({"role":"tool","tool_call_id":b["tool_use_id"].as_str().ok_or("missing tool result ID")?,"content":if let Some(c)=b.get("content") { text(c)? } else { String::new() }})),
                // Anthropic signatures are not valid NVIDIA reasoning state.
                Some("thinking" | "redacted_thinking") if role == "assistant" => {},
                _ => return Err("unsupported content for NVIDIA; use text, images and function tools"),
            }
        }
        // OpenAI requires tool results immediately after the assistant call.
        messages.extend(results);
        if !content.is_empty() || !calls.is_empty() {
            let mut message = json!({"role":role,"content":content});
            if !calls.is_empty() {
                message["tool_calls"] = json!(calls);
            }
            messages.push(message);
        }
    }
    if !system_parts.is_empty() {
        messages.insert(
            0,
            json!({"role":"system", "content":system_parts.join("\n\n")}),
        );
    }
    let mut out = json!({"model":model,"messages":messages,"max_tokens":v["max_tokens"].as_u64().unwrap_or(16384).clamp(1,32768),"stream":v["stream"].as_bool().unwrap_or(false)});
    if out["stream"] == true {
        out["stream_options"] = json!({"include_usage":true});
    }
    for key in ["temperature", "top_p"] {
        if let Some(value) = v.get(key) {
            out[key] = value.clone();
        }
    }
    if let Some(stop) = v.get("stop_sequences") {
        out["stop"] = stop.clone();
    }
    if let Some(tools) = v.get("tools") {
        let mut mapped = Vec::new();
        for tool in tools.as_array().ok_or("invalid tools")? {
            if tool.get("type").is_some_and(|t| t != "custom") {
                return Err(
                    "NVIDIA supports function tools; Anthropic server/computer tools are unavailable",
                );
            }
            mapped.push(json!({"type":"function","function":{"name":tool["name"].as_str().ok_or("missing tool name")?,"description":tool["description"].as_str().unwrap_or(""),"parameters":tool["input_schema"]}}));
        }
        if !mapped.is_empty() {
            out["tools"] = json!(mapped);
        }
    }
    if let Some(choice) = v.get("tool_choice") {
        out["tool_choice"] = match choice["type"].as_str() {
            Some("auto") => json!("auto"),
            Some("any") => json!("required"),
            Some("none") => json!("none"),
            Some("tool") => {
                json!({"type":"function","function":{"name":choice["name"].as_str().ok_or("missing selected tool")?}})
            }
            _ => return Err("unsupported tool choice"),
        };
        if let Some(disabled) = choice["disable_parallel_tool_use"].as_bool() {
            out["parallel_tool_calls"] = json!(!disabled);
        }
    }
    if model.contains("nemotron-3") {
        out["chat_template_kwargs"] =
            json!({"enable_thinking":false,"force_nonempty_content":true});
    }
    Ok(out)
}
fn stop_reason(s: &str) -> Result<&'static str, io::Error> {
    match s {
        "stop" => Ok("end_turn"),
        "length" => Ok("max_tokens"),
        "tool_calls" => Ok("tool_use"),
        "content_filter" => Ok("refusal"),
        _ => Err(invalid()),
    }
}
fn usage(v: &Value) -> Value {
    json!({"input_tokens":v["prompt_tokens"].as_u64().unwrap_or(0),"output_tokens":v["completion_tokens"].as_u64().unwrap_or(0)})
}
pub fn response(v: &Value) -> io::Result<Value> {
    let choice = &v["choices"][0];
    let m = &choice["message"];
    let mut content = Vec::new();
    if let Some(t) = m["content"].as_str().filter(|t| !t.is_empty()) {
        content.push(json!({"type":"text","text":t}));
    }
    if let Some(calls) = m["tool_calls"].as_array() {
        for c in calls {
            let args: Value =
                serde_json::from_str(c["function"]["arguments"].as_str().ok_or_else(invalid)?)
                    .map_err(|_| invalid())?;
            if !args.is_object() {
                return Err(invalid());
            }
            content.push(json!({"type":"tool_use","id":c["id"].as_str().ok_or_else(invalid)?,"name":c["function"]["name"].as_str().ok_or_else(invalid)?,"input":args}));
        }
    }
    Ok(
        json!({"id":v["id"].as_str().unwrap_or("msg_nvidia"),"type":"message","role":"assistant","model":v["model"],"content":content,"stop_reason":stop_reason(choice["finish_reason"].as_str().ok_or_else(invalid)?)?,"stop_sequence":null,"usage":usage(&v["usage"])}),
    )
}
fn chunk(w: &mut impl Write, bytes: &[u8]) -> io::Result<()> {
    write!(w, "{:x}\r\n", bytes.len())?;
    w.write_all(bytes)?;
    w.write_all(b"\r\n")?;
    w.flush()
}
fn event(w: &mut impl Write, v: Value) -> io::Result<()> {
    let data = format!(
        "event: {}\ndata: {}\n\n",
        v["type"].as_str().ok_or_else(invalid)?,
        v
    );
    chunk(w, data.as_bytes())
}
#[derive(Default)]
struct Tool {
    id: String,
    name: String,
    args: String,
    index: Option<usize>,
}
/// Translate incrementally. EOF, malformed frames or invalid tools abort without
/// message_stop or a final HTTP chunk: callers see an interruption, never replay.
pub fn translate_sse(reader: impl Read, w: &mut impl Write, model: &str) -> io::Result<()> {
    let mut reader = BufReader::new(reader);
    let mut data = String::new();
    let mut started = false;
    let mut tools: BTreeMap<u64, Tool> = BTreeMap::new();
    let mut text_index = None;
    let mut next = 0usize;
    let mut finish = None;
    let mut tokens = json!({"input_tokens":0,"output_tokens":0});
    let mut bytes = 0usize;
    let mut done = false;
    loop {
        let mut line = String::new();
        let n = reader
            .by_ref()
            .take((1024 * 1024 + 1) as u64)
            .read_line(&mut line)?;
        if n == 0 {
            break;
        }
        bytes += n;
        if n > 1024 * 1024 || bytes > LIMIT {
            return Err(invalid());
        }
        if line.trim().is_empty() {
            if data.is_empty() {
                continue;
            }
            if data.trim() == "[DONE]" {
                done = true;
                break;
            }
            let v: Value = serde_json::from_str(&data).map_err(|_| invalid())?;
            data.clear();
            if v.get("error").is_some() {
                return Err(invalid());
            }
            if !started {
                event(
                    w,
                    json!({"type":"message_start","message":{"id":v["id"].as_str().unwrap_or("msg_nvidia"),"type":"message","role":"assistant","model":model,"content":[],"stop_reason":null,"stop_sequence":null,"usage":{"input_tokens":0,"output_tokens":0}}}),
                )?;
                started = true;
            }
            if !v["usage"].is_null() {
                tokens = usage(&v["usage"]);
            }
            for choice in v["choices"].as_array().ok_or_else(invalid)? {
                if choice["index"].as_u64().unwrap_or(0) != 0 {
                    return Err(invalid());
                }
                let delta = &choice["delta"];
                if let Some(t) = delta["content"].as_str().filter(|t| !t.is_empty()) {
                    let idx = if let Some(idx) = text_index {
                        idx
                    } else {
                        let idx = next;
                        next += 1;
                        text_index = Some(idx);
                        event(
                            w,
                            json!({"type":"content_block_start","index":idx,"content_block":{"type":"text","text":""}}),
                        )?;
                        idx
                    };
                    event(
                        w,
                        json!({"type":"content_block_delta","index":idx,"delta":{"type":"text_delta","text":t}}),
                    )?;
                }
                if let Some(calls) = delta["tool_calls"].as_array() {
                    for c in calls {
                        let index = c["index"].as_u64().ok_or_else(invalid)?;
                        if index >= 128 {
                            return Err(invalid());
                        }
                        let tool = tools.entry(index).or_default();
                        if let Some(id) = c["id"].as_str() {
                            tool.id.push_str(id);
                        }
                        if let Some(name) = c["function"]["name"].as_str().filter(|s| !s.is_empty())
                        {
                            if tool.index.is_some() {
                                return Err(invalid());
                            }
                            tool.name.push_str(name);
                        }
                        if tool.id.len() > 1024 || tool.name.len() > 256 {
                            return Err(invalid());
                        }
                        if let Some(args) = c["function"]["arguments"].as_str() {
                            tool.args.push_str(args);
                            if tool.args.len() > 2 * 1024 * 1024 {
                                return Err(invalid());
                            }
                            if tool.index.is_none() && !tool.id.is_empty() && !tool.name.is_empty()
                            {
                                let idx = next;
                                next += 1;
                                tool.index = Some(idx);
                                event(
                                    w,
                                    json!({"type":"content_block_start","index":idx,"content_block":{"type":"tool_use","id":tool.id,"name":tool.name,"input":{}}}),
                                )?;
                                if !tool.args.is_empty() {
                                    event(
                                        w,
                                        json!({"type":"content_block_delta","index":idx,"delta":{"type":"input_json_delta","partial_json":tool.args}}),
                                    )?;
                                }
                            } else if let Some(idx) = tool.index
                                && !args.is_empty()
                            {
                                event(
                                    w,
                                    json!({"type":"content_block_delta","index":idx,"delta":{"type":"input_json_delta","partial_json":args}}),
                                )?;
                            }
                        }
                    }
                }
                if let Some(reason) = choice["finish_reason"].as_str() {
                    finish = Some(stop_reason(reason)?);
                }
            }
        } else if let Some(part) = line.strip_prefix("data:") {
            if !data.is_empty() {
                data.push('\n');
            }
            data.push_str(part.trim_start().trim_end_matches(['\r', '\n']));
        }
    }
    if !done || !started || finish.is_none() {
        return Err(invalid());
    }
    for tool in tools.values() {
        let v: Value = serde_json::from_str(&tool.args).map_err(|_| invalid())?;
        if !v.is_object() || tool.index.is_none() {
            return Err(invalid());
        }
    }
    for idx in 0..next {
        event(w, json!({"type":"content_block_stop","index":idx}))?;
    }
    event(
        w,
        json!({"type":"message_delta","delta":{"stop_reason":finish,"stop_sequence":null},"usage":tokens}),
    )?;
    event(w, json!({"type":"message_stop"}))
}
pub fn write_response(
    stream: &mut TcpStream,
    resp: &mut ureq::http::Response<ureq::Body>,
    streaming: bool,
    model: &str,
) -> io::Result<()> {
    let ctype = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_owned();
    if streaming && ctype.contains("text/event-stream") {
        stream.write_all(b"HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ntransfer-encoding: chunked\r\nconnection: close\r\n\r\n")?;
        translate_sse(resp.body_mut().as_reader(), stream, model)?;
        stream.write_all(b"0\r\n\r\n")?;
        return stream.flush();
    }
    let mut raw = String::new();
    resp.body_mut()
        .as_reader()
        .take((LIMIT + 1) as u64)
        .read_to_string(&mut raw)?;
    if raw.len() > LIMIT {
        return Err(invalid());
    }
    let v: Value = serde_json::from_str(&raw).map_err(|_| invalid())?;
    let mut message = response(&v)?;
    message["model"] = json!(model);
    if streaming {
        return Err(invalid());
    }
    let bytes = message.to_string();
    write!(
        stream,
        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
        bytes.len(),
        bytes
    )?;
    stream.flush()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn maps_function_calls_results_images_and_choice_without_orphaning_results() {
        let body = json!({"model":"nvidia/nemotron-3-ultra-550b-a55b","max_tokens":64000,"stream":true,"system":[{"type":"text","text":"System"}],"tools":[{"name":"lookup","input_schema":{"type":"object"}}],"tool_choice":{"type":"tool","name":"lookup","disable_parallel_tool_use":true},"messages":[{"role":"assistant","content":[{"type":"tool_use","id":"call1","name":"lookup","input":{"q":"x"}}]},{"role":"user","content":[{"type":"text","text":"Continue"},{"type":"tool_result","tool_use_id":"call1","content":[{"type":"text","text":"Result"}]},{"type":"image","source":{"type":"base64","media_type":"image/png","data":"AAAA"}}]}]});
        let wire = request(&serde_json::to_vec(&body).unwrap(), None).unwrap();
        assert_eq!(wire["messages"][0]["role"], "system");
        assert_eq!(
            wire["messages"][1]["tool_calls"][0]["function"]["arguments"],
            "{\"q\":\"x\"}"
        );
        assert_eq!(wire["messages"][2]["role"], "tool");
        assert_eq!(wire["messages"][2]["tool_call_id"], "call1");
        assert_eq!(
            wire["messages"][3]["content"][1]["image_url"]["url"],
            "data:image/png;base64,AAAA"
        );
        assert_eq!(wire["parallel_tool_calls"], false);
        assert_eq!(wire["max_tokens"], 32768);
        assert_eq!(wire["chat_template_kwargs"]["force_nonempty_content"], true);
    }
    fn sse(chunks: Vec<Value>, done: bool) -> String {
        let mut s = chunks
            .into_iter()
            .map(|v| format!("data: {v}\n\n"))
            .collect::<String>();
        if done {
            s.push_str("data: [DONE]\n\n");
        }
        s
    }
    #[test]
    fn streamed_parallel_tool_fragments_and_usage_are_preserved() {
        let input = sse(
            vec![
                json!({"id":"chat-1","choices":[{"index":0,"delta":{"content":"Checking", "tool_calls":[{"index":0,"id":"a","function":{"name":"one","arguments":"{\"x\":"}},{"index":1,"id":"b","function":{"name":"two","arguments":"{}"}}]},"finish_reason":null}]}),
                json!({"choices":[{"delta":{"tool_calls":[{"index":0,"function":{"arguments":"42}"}}]},"finish_reason":"tool_calls"}]}),
                json!({"choices":[],"usage":{"prompt_tokens":12,"completion_tokens":9}}),
            ],
            true,
        );
        let mut out = Vec::new();
        translate_sse(input.as_bytes(), &mut out, "model").unwrap();
        let out = String::from_utf8(out).unwrap();
        assert!(out.contains("input_json_delta"));
        assert!(out.contains("tool_use"));
        assert!(out.contains("Checking"));
        assert!(out.contains("\"input_tokens\":12"));
        assert!(out.contains("message_stop"));
        assert!(out.contains("42}"));
    }
    #[test]
    fn truncated_or_malformed_stream_does_not_emit_successful_stop() {
        for data in [
            sse(
                vec![json!({"choices":[{"delta":{"content":"partial"},"finish_reason":null}]})],
                false,
            ),
            "data: not json\n\n".into(),
            sse(
                vec![
                    json!({"choices":[{"delta":{"tool_calls":[{"index":9999}]},"finish_reason":"tool_calls"}]}),
                ],
                true,
            ),
        ] {
            let mut out = Vec::new();
            assert!(translate_sse(data.as_bytes(), &mut out, "model").is_err());
            assert!(!String::from_utf8(out).unwrap().contains("message_stop"));
        }
    }
    #[test]
    fn nonstream_tools_round_trip_into_next_request() {
        let v = json!({"id":"c","model":"nvidia","choices":[{"message":{"content":null,"tool_calls":[{"id":"call","function":{"name":"add","arguments":"{\"x\":1}"}}]},"finish_reason":"tool_calls"}],"usage":{"prompt_tokens":4,"completion_tokens":3}});
        let mapped = response(&v).unwrap();
        assert_eq!(mapped["stop_reason"], "tool_use");
        assert_eq!(mapped["content"][0]["input"]["x"], 1);
        let body = json!({"model":"nvidia","messages":[{"role":"assistant","content":mapped["content"]},{"role":"user","content":[{"type":"tool_result","tool_use_id":"call","content":"2"}]}]});
        let wire = request(&serde_json::to_vec(&body).unwrap(), None).unwrap();
        assert_eq!(wire["messages"][1]["tool_call_id"], "call");
    }
    #[test]
    fn documents_and_server_tools_are_rejected_instead_of_dropped() {
        for body in [
            json!({"model":"nvidia","messages":[{"role":"user","content":[{"type":"document"}]}]}),
            json!({"model":"nvidia","messages":[],"tools":[{"type":"web_search_20250305","name":"web_search"}]}),
        ] {
            assert!(request(&body.to_string().into_bytes(), None).is_err());
        }
    }
}

#[cfg(test)]
mod system_message_tests {
    use super::*;
    fn translate(messages: Value) -> Result<Value, &'static str> {
        request(
            &serde_json::to_vec(&json!({"system":"Initial instructions", "messages":messages}))
                .unwrap(),
            Some("nvidia/nemotron-3-ultra-550b-a55b"),
        )
    }
    #[test]
    fn inline_system_instructions_are_hoisted_in_order_without_changing_tool_pairs() {
        let out=translate(json!([
            {"role":"user","content":"Read the fixture"},
            {"role":"assistant","content":[{"type":"tool_use","id":"call1","name":"Read","input":{}}]},
            {"role":"user","content":[{"type":"tool_result","tool_use_id":"call1","content":"fixture result"}]},
            {"role":"system","content":[{"type":"text","text":"Later instructions"}]},
            {"role":"system","clear_at":"never","content":"Latest instructions"}
        ])).unwrap();
        let messages = out["messages"].as_array().unwrap();
        assert_eq!(
            messages[0],
            json!({"role":"system","content":"Initial instructions\n\nLater instructions\n\nLatest instructions"})
        );
        assert_eq!(messages.iter().filter(|m| m["role"] == "system").count(), 1);
        assert_eq!(messages[2]["tool_calls"][0]["id"], "call1");
        assert_eq!(
            messages[3],
            json!({"role":"tool","tool_call_id":"call1","content":"fixture result"})
        );
    }
    #[test]
    fn turn_scoped_instructions_clear_on_later_user_including_tool_results() {
        let out = translate(json!([
            {"role":"user","content":"First request"},
            {"role":"system","clear_at":"next_user_message","content":"Expired reminder"},
            {"role":"assistant","content":[{"type":"tool_use","id":"t","name":"Read","input":{}}]},
            {"role":"user","content":[{"type":"tool_result","tool_use_id":"t","content":"Done"}]},
            {"role":"system","clear_at":"next_user_message","content":"Active reminder"}
        ]))
        .unwrap();
        assert_eq!(
            out["messages"][0]["content"],
            "Initial instructions\n\nActive reminder"
        );
        assert!(!out.to_string().contains("Expired reminder"));
    }
    #[test]
    fn persistent_instructions_survive_later_turns_and_assistant_does_not_clear_reminders() {
        let out = translate(json!([
            {"role":"user","content":"Start"},
            {"role":"system","clear_at":"never","content":"Persistent"},
            {"role":"assistant","content":"Reply"},
            {"role":"user","content":"Continue"},
            {"role":"system","clear_at":"next_user_message","content":"Still active"},
            {"role":"assistant","content":"Partial"}
        ]))
        .unwrap();
        assert_eq!(
            out["messages"][0]["content"],
            "Initial instructions\n\nPersistent\n\nStill active"
        );
    }
    #[test]
    fn system_only_request_does_not_require_a_top_level_system_field() {
        let out=request(&serde_json::to_vec(&json!({"messages":[{"role":"user","content":"Reply"},{"role":"system","content":"Be brief"}]})).unwrap(),Some("nvidia/model")).unwrap();
        assert_eq!(
            out["messages"][0],
            json!({"role":"system","content":"Be brief"})
        );
        assert_eq!(out["messages"][1]["role"], "user");
    }
    #[test]
    fn unsupported_system_features_fail_before_sending_instead_of_being_dropped() {
        for message in [
            json!({"role":"system","content":[{"type":"tool_addition","tool":{"type":"tool_reference","name":"Read"}}]}),
            json!({"role":"system","content":"Notice","output_config":{"effort":"high"}}),
            json!({"role":"system","content":"Notice","clear_at":"invented"}),
            json!({"role":"user","content":"Notice","clear_at":"never"}),
        ] {
            assert!(translate(json!([{"role":"user","content":"Start"},message])).is_err());
        }
    }
    #[test]
    fn unsupported_roles_do_not_echo_arbitrary_values() {
        let err = translate(json!([{"role":"private-role-value","content":"private-content"}]))
            .unwrap_err();
        assert!(!err.contains("private-role-value"));
        assert!(!err.contains("private-content"));
    }
}

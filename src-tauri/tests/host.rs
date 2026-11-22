//! The agent host end to end, with a fake `claude` that speaks stream-json
//! (no model calls): conversations stream to a client, keep working while
//! no client is attached, replay on attach, drain queued messages, and the
//! host stops idle work and exits once nobody needs it.
#![cfg(unix)]

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::{Child, Command};
use std::time::{Duration, Instant};

use serde_json::{json, Value};

const FAKE_CLAUDE: &str = r#"#!/usr/bin/env python3
import json, sys, time
sid = "fake-session"
def out(o):
    sys.stdout.write(json.dumps(o) + "\n"); sys.stdout.flush()
n = 0
for line in sys.stdin:
    try:
        m = json.loads(line)
    except Exception:
        continue
    if m.get("type") == "control_request":
        out({"type": "control_response", "response": {"subtype": "success", "request_id": m.get("request_id"), "response": {"commands": [], "agents": [], "models": []}}})
        continue
    if m.get("type") != "user":
        continue
    content = m.get("message", {}).get("content", [])
    text = " ".join(p.get("text", "") for p in content if isinstance(p, dict) and p.get("type") == "text") if isinstance(content, list) else str(content)
    n += 1
    out({"type": "system", "subtype": "init", "session_id": sid, "model": "fake", "tools": [], "cwd": "."})
    time.sleep(3.0 if "slow" in text else 1.0)
    out({"type": "assistant", "session_id": sid, "parent_tool_use_id": None, "uuid": "a%d" % n,
         "message": {"id": "msg_%d" % n, "type": "message", "role": "assistant", "model": "fake",
                     "content": [{"type": "text", "text": "echo: " + text}], "stop_reason": "end_turn",
                     "usage": {"input_tokens": 1, "output_tokens": 1}}})
    out({"type": "result", "subtype": "success", "is_error": False, "duration_ms": 1000, "num_turns": 1,
         "result": "echo: " + text, "session_id": sid, "total_cost_usd": 0.0,
         "usage": {"input_tokens": 1, "output_tokens": 1}})
"#;

struct Env {
    dir: PathBuf,
}

impl Env {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("gh-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("home")).unwrap();
        let fake = dir.join("claude");
        std::fs::write(&fake, FAKE_CLAUDE).unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();
        Self { dir }
    }

    fn spawn_host(&self) -> Child {
        Command::new(env!("CARGO_BIN_EXE_glaude"))
            .arg("--host")
            .env("GLAUDE_HOST_DIR", &self.dir)
            .env("GLAUDE_HOST_QUIET_SECS", "2")
            .env("HOME", self.dir.join("home"))
            .env("SHELL", "/bin/sh")
            .env("CLAUDE_BIN", self.dir.join("claude"))
            .spawn()
            .expect("start host")
    }

    fn connect(&self) -> Client {
        let sock = self.dir.join("host.sock");
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if let Ok(s) = UnixStream::connect(&sock) {
                let token = std::fs::read_to_string(self.dir.join("host.token")).unwrap();
                let mut c = Client::new(s);
                let hello = c.call(
                    "hello",
                    json!({"token": token.trim(), "proto": 1, "stamp": ""}),
                );
                assert!(hello.get("error").is_none(), "{hello}");
                return c;
            }
            assert!(Instant::now() < deadline, "host did not start");
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    fn cwd(&self) -> String {
        self.dir.display().to_string()
    }
}

impl Drop for Env {
    fn drop(&mut self) {
        if std::thread::panicking() {
            if let Ok(log) = std::fs::read_to_string(self.dir.join("host.log")) {
                eprintln!("--- host.log ---\n{log}");
            }
        }
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

struct Client {
    w: UnixStream,
    r: BufReader<UnixStream>,
    next: u64,
    /// Event lines seen so far: (conv, events).
    events: Vec<(String, Vec<Value>)>,
}

impl Client {
    fn new(s: UnixStream) -> Self {
        s.set_read_timeout(Some(Duration::from_millis(200)))
            .unwrap();
        Self {
            r: BufReader::new(s.try_clone().unwrap()),
            w: s,
            next: 0,
            events: Vec::new(),
        }
    }

    fn read(&mut self) -> Option<Value> {
        let mut line = String::new();
        match self.r.read_line(&mut line) {
            Ok(0) => None,
            Ok(_) => serde_json::from_str(&line).ok(),
            Err(_) => None,
        }
    }

    fn call(&mut self, method: &str, params: Value) -> Value {
        let id = self.next;
        self.next += 1;
        let line = json!({"id": id, "method": method, "params": params}).to_string();
        self.w.write_all(format!("{line}\n").as_bytes()).unwrap();
        let deadline = Instant::now() + Duration::from_secs(20);
        while Instant::now() < deadline {
            if let Some(v) = self.read() {
                if let Some(conv) = v.get("conv").and_then(Value::as_str) {
                    let evs = v["events"].as_array().cloned().unwrap_or_default();
                    self.events.push((conv.to_string(), evs));
                } else if v.get("id").and_then(Value::as_u64) == Some(id) {
                    return v;
                }
            }
        }
        panic!("no reply to {method}");
    }

    /// Read events until `pred` matches one (returns it).
    fn wait_event(&mut self, secs: u64, pred: impl Fn(&Value) -> bool) -> Value {
        if let Some(e) = self.events.iter().flat_map(|(_, e)| e).find(|e| pred(e)) {
            return e.clone();
        }
        let deadline = Instant::now() + Duration::from_secs(secs);
        while Instant::now() < deadline {
            if let Some(v) = self.read() {
                if let Some(conv) = v.get("conv").and_then(Value::as_str) {
                    let evs = v["events"].as_array().cloned().unwrap_or_default();
                    let hit = evs.iter().find(|e| pred(e)).cloned();
                    self.events.push((conv.to_string(), evs));
                    if let Some(h) = hit {
                        return h;
                    }
                }
            }
        }
        panic!("event not seen");
    }
}

fn assistant_says(text: &'static str) -> impl Fn(&Value) -> bool {
    move |e| {
        e["type"] == "item"
            && e["item"]["kind"] == "assistant"
            && e["item"]["streaming"] == false
            && e["item"]["blocks"].to_string().contains(text)
    }
}

fn wait_exit(child: &mut Child, secs: u64) -> bool {
    let deadline = Instant::now() + Duration::from_secs(secs);
    while Instant::now() < deadline {
        if child.try_wait().unwrap().is_some() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    false
}

fn start(c: &mut Client, conv: &str, cwd: &str) {
    let r = c.call(
        "start",
        json!({"convId": conv, "spec": {"provider": "claude", "account": "default", "cwd": cwd}}),
    );
    assert!(r.get("error").is_none(), "{r}");
}

#[test]
fn streams_queues_replays_and_exits_when_idle() {
    let env = Env::new("idle");
    let mut host = env.spawn_host();
    let mut c = env.connect();
    start(&mut c, "c1", &env.cwd());

    c.call(
        "send",
        json!({"convId": "c1", "text": "hello", "attachments": []}),
    );
    c.wait_event(15, assistant_says("echo: hello"));

    // A message queued while a turn runs goes out when the turn ends.
    c.call(
        "send",
        json!({"convId": "c1", "text": "first", "attachments": []}),
    );
    c.call(
        "queue",
        json!({"convId": "c1", "items": [{"id": "q1", "text": "second"}]}),
    );
    c.wait_event(15, |e| {
        e["type"] == "queue" && e["items"].as_array().is_some_and(|a| a.len() == 1)
    });
    c.wait_event(15, assistant_says("echo: first"));
    c.wait_event(15, |e| {
        e["type"] == "queue" && e["items"].as_array().is_some_and(|a| a.is_empty())
    });
    c.wait_event(15, assistant_says("echo: second"));

    // A new client (another window, or the app after a reload) gets the
    // whole conversation on attach.
    let mut c2 = env.connect();
    let list = c2.call("list", json!({}));
    assert_eq!(list["ok"][0]["convId"], "c1", "{list}");
    c2.call("attach", json!({"convId": "c1"}));
    let reset = c2.wait_event(5, |e| e["type"] == "reset");
    let all = reset.to_string();
    assert!(
        all.contains("echo: hello") && all.contains("echo: second"),
        "{all}"
    );

    // Nobody attached and nothing to do: the host stops the conversation and exits.
    drop(c);
    drop(c2);
    assert!(wait_exit(&mut host, 20), "host should exit when idle");
    assert!(!env.dir.join("host.sock").exists());
}

#[test]
fn keeps_working_with_no_window() {
    let env = Env::new("busy");
    let mut host = env.spawn_host();
    let mut c = env.connect();
    start(&mut c, "c1", &env.cwd());
    c.call(
        "send",
        json!({"convId": "c1", "text": "slow job", "attachments": []}),
    );
    // Close the "window" while the turn runs.
    drop(c);
    std::thread::sleep(Duration::from_millis(1000));
    assert!(
        host.try_wait().unwrap().is_none(),
        "host keeps running a busy turn"
    );

    // Reopen mid-turn: the state so far, then the rest of the turn live.
    let mut c2 = env.connect();
    let brief = c2.call("attach", json!({"convId": "c1"}));
    assert_eq!(brief["ok"]["busy"], true, "{brief}");
    let reset = c2.wait_event(5, |e| e["type"] == "reset");
    assert!(reset.to_string().contains("slow job"), "{reset}");
    c2.wait_event(10, assistant_says("echo: slow job"));
    drop(c2);
    assert!(wait_exit(&mut host, 20));
}

#[test]
fn refuses_a_wrong_token() {
    let env = Env::new("token");
    let mut host = env.spawn_host();
    let _ = env.connect(); // waits for the socket
    let mut s = UnixStream::connect(env.dir.join("host.sock")).unwrap();
    s.write_all(b"{\"id\":0,\"method\":\"hello\",\"params\":{\"token\":\"nope\"}}\n")
        .unwrap();
    let mut r = BufReader::new(s);
    let mut line = String::new();
    r.read_line(&mut line).unwrap();
    assert!(line.contains("bad token"), "{line}");
    let _ = host.kill();
    let _ = host.wait();
}

#[test]
fn scheduled_resume_sends_continue() {
    let env = Env::new("sched");
    let mut host = env.spawn_host();
    let mut c = env.connect();
    start(&mut c, "c1", &env.cwd());
    c.call(
        "send",
        json!({"convId": "c1", "text": "hello", "attachments": []}),
    );
    c.wait_event(15, assistant_says("echo: hello"));

    let at = chrono::Utc::now() + chrono::Duration::seconds(2);
    let r = c.call(
        "schedule",
        json!({"convId": "c1", "resumeAt": at.to_rfc3339()}),
    );
    assert!(r.get("error").is_none(), "{r}");
    c.wait_event(5, |e| e["type"] == "schedule" && e["resumeAt"].is_string());
    // The host continues on its own at that time, then clears the schedule.
    c.wait_event(20, assistant_says("echo: continue"));
    let cleared = c
        .events
        .iter()
        .flat_map(|(_, e)| e)
        .any(|e| e["type"] == "schedule" && e["resumeAt"].is_null());
    assert!(cleared, "schedule cleared");
    drop(c);
    assert!(wait_exit(&mut host, 20));
}

#[test]
fn close_all_stops_running_agents() {
    let env = Env::new("closeall");
    let mut host = env.spawn_host();
    let mut c = env.connect();
    let r = c.call("config", json!({"notify": false}));
    assert!(r.get("error").is_none(), "{r}");
    start(&mut c, "c1", &env.cwd());
    c.call("send", json!({"convId": "c1", "text": "slow job", "attachments": []}));
    c.wait_event(10, |e| e["type"] == "turn" && e["state"] != "idle");
    // Quitting with "keep agents working" off.
    c.call("close_all", json!({}));
    let list = c.call("list", json!({}));
    assert_eq!(list["ok"], json!([]), "{list}");
    drop(c);
    assert!(wait_exit(&mut host, 20), "nothing left: the host exits");
}

//! Refresh usage for every account under a HOME and print usage + profile
//! as JSON. Used for the parity check against account_manager.py:
//!
//!     cargo run -p glaude-accounts --example dump -- <home>
//!     cargo run -p glaude-accounts --example dump -- <home> list
//!     cargo run -p glaude-accounts --example dump -- <home> share <tool> <name>

use std::path::PathBuf;

use glaude_accounts::{claude, codex, read_profile, Ctx, Kind};
use serde_json::{json, Map, Value};

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let home = PathBuf::from(
        args.get(1)
            .expect("usage: dump <home> [share <tool> <name>]"),
    );
    let ctx = Ctx::with_home(&home);
    if args.get(2).map(String::as_str) == Some("list") {
        // Read-only: what the Accounts page shows before any scan.
        let mut all = Vec::new();
        for kind in [Kind::Claude, Kind::Codex] {
            all.extend(glaude_accounts::list(&ctx, kind));
        }
        println!("{}", serde_json::to_string_pretty(&all)?);
        return Ok(());
    }
    if args.get(2).map(String::as_str) == Some("share") {
        let kind = Kind::parse(&args[3])?;
        let mut acct = kind.find(&ctx, &args[4])?;
        let log = glaude_accounts::share::share(&ctx, kind, &mut acct, false)?;
        println!("{}", serde_json::to_string_pretty(&log)?);
        return Ok(());
    }
    let mut out = Map::new();
    for kind in [Kind::Claude, Kind::Codex] {
        let mut per = Map::new();
        for acct in kind.accounts(&ctx, true) {
            let usage = match kind {
                Kind::Claude => claude::refresh_usage(&ctx, &acct, &mut |_| {})?,
                Kind::Codex => codex::refresh_usage(&ctx, &acct, &mut |_| {})?,
            };
            let acct = kind.find(&ctx, &acct.name)?;
            per.insert(
                acct.name.clone(),
                json!({"usage": usage, "profile": read_profile(&ctx, &acct)}),
            );
        }
        out.insert(kind.as_str().into(), Value::Object(per));
    }
    println!("{}", serde_json::to_string_pretty(&Value::Object(out))?);
    Ok(())
}

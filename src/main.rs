//! Thin CLI wrapper around the pure-Rust `clearway` library.
//!
//! Usage:
//!   `clearway <html-or-ir-json|-> [--ir] [--json] [--sarif] [--rules] [--tags <list>] [--all-rules] [--profile <standard|act>] [--bench <n>]`

use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::hint::black_box;
use std::io::{self, Read};
use std::process::ExitCode;
use std::time::Instant;

use clearway::{
    AuditOptions, AuditReport, BUILTIN_RULES, ConformanceProfile, Document, RuleLevel,
    audit_document, audit_html, to_sarif_json,
};

#[allow(clippy::struct_excessive_bools)]
struct CliArgs {
    input: Option<String>,
    is_ir: bool,
    json: bool,
    sarif: bool,
    list_rules: bool,
    tags: Option<Vec<String>>,
    all_rules: bool,
    profile: ConformanceProfile,
    bench: Option<usize>,
}

fn parse_args() -> Result<CliArgs, String> {
    let mut args = env::args().skip(1);
    let mut parsed = CliArgs {
        input: None,
        is_ir: false,
        json: false,
        sarif: false,
        list_rules: false,
        tags: None,
        all_rules: false,
        profile: ConformanceProfile::Standard,
        bench: None,
    };

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--ir" => parsed.is_ir = true,
            "--json" => parsed.json = true,
            "--sarif" => parsed.sarif = true,
            "--rules" => parsed.list_rules = true,
            "--all-rules" => parsed.all_rules = true,
            "--tags" => {
                let Some(val) = args.next() else {
                    return Err("--tags requires a comma-separated list of tags".to_owned());
                };
                let tags: Vec<String> = val
                    .split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_owned)
                    .collect();
                if tags.is_empty() {
                    return Err("--tags requires at least one non-empty tag".to_owned());
                }
                parsed.tags = Some(tags);
            }
            "--profile" => {
                let Some(val) = args.next() else {
                    return Err("--profile requires 'standard' or 'act'".to_owned());
                };
                parsed.profile = match val.trim().to_ascii_lowercase().as_str() {
                    "standard" => ConformanceProfile::Standard,
                    "act" | "w3cact" | "w3c-act" => ConformanceProfile::W3cAct,
                    _ => {
                        return Err(format!(
                            "invalid --profile value '{val}' (expected 'standard' or 'act')"
                        ));
                    }
                };
            }
            "--bench" => {
                let Some(val) = args.next() else {
                    return Err("--bench requires a positive integer iteration count".to_owned());
                };
                let n = val
                    .parse::<usize>()
                    .map_err(|_| format!("invalid --bench value '{val}'"))?;
                if n == 0 {
                    return Err("--bench iteration count must be at least 1".to_owned());
                }
                parsed.bench = Some(n);
            }
            "-h" | "--help" => {
                println!("{USAGE}");
                std::process::exit(0);
            }
            other if other.starts_with('-') && other != "-" => {
                return Err(format!("unknown flag '{other}'"));
            }
            path => {
                if parsed.input.is_some() {
                    return Err(format!("unexpected extra argument '{path}'"));
                }
                parsed.input = Some(path.to_owned());
            }
        }
    }

    Ok(parsed)
}

const USAGE: &str = "\
Usage: clearway <html-or-ir-json|-> [--ir] [--json] [--sarif] [--rules] [--tags <list>] [--all-rules] [--profile <standard|act>] [--bench <n>] [-h|--help]

Options:
  --ir                  Parse input as serialized Document IR JSON instead of HTML
  --json                Emit machine-readable JSON output
  --sarif               Emit SARIF 2.1.0 JSON for GitHub Code Scanning / PR annotations
  --rules               List all built-in accessibility rules and exit
  --tags <comma-list>   Run only rules matching at least one tag (e.g. wcag2a,wcag2aa)
  --all-rules           Include rules that are disabled by default
  --profile <standard|act>  Profile: 'standard' (default, flags placeholder-only inputs) or 'act' (allows placeholder fallback on label)
  --bench <n>           Run <n> audit iterations and report timing (us/audit)
  -h, --help            Print usage information and exit";

fn read_input(path: &str) -> Result<String, String> {
    if path == "-" {
        let mut buf = String::new();
        io::stdin()
            .read_to_string(&mut buf)
            .map_err(|e| format!("failed to read stdin: {e}"))?;
        Ok(buf)
    } else {
        fs::read_to_string(path).map_err(|e| format!("failed to read '{path}': {e}"))
    }
}

#[allow(clippy::cast_precision_loss)]
fn run_bench(raw: &str, is_ir: bool, iterations: usize, options: &AuditOptions) -> ExitCode {
    if is_ir {
        let doc: Document = match serde_json::from_str(raw) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("error: invalid IR JSON: {e}");
                return ExitCode::from(2);
            }
        };
        let _ = black_box(audit_document(black_box(&doc), options));
        let start = Instant::now();
        for _ in 0..iterations {
            let report = audit_document(black_box(&doc), options);
            black_box(report.summary.violations);
        }
        let elapsed = start.elapsed();
        let us_per_audit = elapsed.as_secs_f64() * 1_000_000.0 / (iterations as f64);
        let audits_per_sec = if us_per_audit > 0.0 {
            1_000_000.0 / us_per_audit
        } else {
            0.0
        };
        println!(
            "bench (IR): {iterations} audits in {:.2} ms ({us_per_audit:.2} us/audit, {audits_per_sec:.0} audits/sec)",
            elapsed.as_secs_f64() * 1000.0,
        );
    } else {
        let _ = black_box(audit_html(black_box(raw), options));
        let start = Instant::now();
        for _ in 0..iterations {
            let report = audit_html(black_box(raw), options);
            black_box(report.summary.violations);
        }
        let elapsed = start.elapsed();
        let us_per_audit = elapsed.as_secs_f64() * 1_000_000.0 / (iterations as f64);
        let audits_per_sec = if us_per_audit > 0.0 {
            1_000_000.0 / us_per_audit
        } else {
            0.0
        };
        println!(
            "bench (HTML5 parse + audit): {iterations} audits in {:.2} ms ({us_per_audit:.2} us/audit, {audits_per_sec:.0} audits/sec)",
            elapsed.as_secs_f64() * 1000.0,
        );
    }
    ExitCode::SUCCESS
}

fn print_text_report(report: &AuditReport) {
    println!(
        "clearway: {} violation(s), {} warning(s), {} incomplete across {} rules ({} elements checked)",
        report.summary.violations,
        report.summary.warnings,
        report.summary.incomplete,
        report.summary.rules_run,
        report.summary.nodes_checked
    );
    for v in &report.violations {
        let tag = match v.level {
            RuleLevel::Error => "ERROR",
            RuleLevel::Warn => "WARN ",
            RuleLevel::Off => "OFF  ",
        };
        println!("  [{tag}] ({:?}) {} — {}", v.impact, v.rule_id, v.help);
        println!("          selector: {}", v.selector);
        println!("          detail:   {}", v.description);
        println!("          snippet:  {}", v.html_snippet);
    }
    for v in &report.incomplete {
        println!("  [REVIEW] ({:?}) {} — {}", v.impact, v.rule_id, v.help);
        println!("           selector: {}", v.selector);
        println!("           detail:   {}", v.description);
        println!("           snippet:  {}", v.html_snippet);
    }
}

fn main() -> ExitCode {
    let args = match parse_args() {
        Ok(a) => a,
        Err(msg) => {
            eprintln!("error: {msg}");
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
    };

    if args.list_rules {
        if args.json {
            match serde_json::to_string_pretty(BUILTIN_RULES) {
                Ok(out) => println!("{out}"),
                Err(e) => {
                    eprintln!("error: failed to serialize rules: {e}");
                    return ExitCode::from(2);
                }
            }
        } else {
            println!("Built-in rules ({}):", BUILTIN_RULES.len());
            for rule in BUILTIN_RULES {
                let default_state = if rule.enabled_by_default { "on" } else { "off" };
                println!(
                    "  {:<24} {:<22} [{:?}] [default:{default_state}] {} [tags: {}]",
                    rule.id,
                    rule.wcag,
                    rule.impact,
                    rule.help,
                    rule.tags.join(", ")
                );
            }
        }
        return ExitCode::SUCCESS;
    }

    let Some(input_path) = args.input else {
        eprintln!("error: missing input path (use '-' for stdin or --rules)");
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };

    let raw = match read_input(&input_path) {
        Ok(s) => s,
        Err(msg) => {
            eprintln!("error: {msg}");
            return ExitCode::from(2);
        }
    };

    let options = AuditOptions {
        rules: BTreeMap::new(),
        tags: args.tags,
        include_disabled: args.all_rules,
        profile: args.profile,
    };

    if let Some(iterations) = args.bench {
        return run_bench(&raw, args.is_ir, iterations, &options);
    }

    let report = if args.is_ir {
        let doc: Document = match serde_json::from_str(&raw) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("error: invalid IR JSON: {e}");
                return ExitCode::from(2);
            }
        };
        audit_document(&doc, &options)
    } else {
        audit_html(&raw, &options)
    };

    if args.sarif {
        let target_uri = if input_path == "-" {
            "stdin"
        } else {
            input_path.as_str()
        };
        match to_sarif_json(&report, target_uri) {
            Ok(out) => println!("{out}"),
            Err(e) => {
                eprintln!("error: failed to serialize SARIF report: {e}");
                return ExitCode::from(2);
            }
        }
    } else if args.json {
        match serde_json::to_string_pretty(&report) {
            Ok(out) => println!("{out}"),
            Err(e) => {
                eprintln!("error: failed to serialize report: {e}");
                return ExitCode::from(2);
            }
        }
    } else {
        print_text_report(&report);
    }

    if report.summary.violations > 0 {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

//! The one line a failed step ends with, so a CI failure is diagnosable from
//! the tail of the step's log even after the raw log has expired.

use std::fmt::Write as _;

/// The longest a quoted line may be.
const MAX_LINE: usize = 200;

/// What a failed step's output says went wrong.
#[derive(Debug, Default, PartialEq, Eq)]
struct Findings {
    /// Failing tests, each with the test binary it ran in.
    failed_tests: Vec<String>,
    /// The first panic: where, and its message.
    first_panic: Option<String>,
    /// The first compiler, rustfmt or cargo complaint.
    first_error: Option<String>,
    /// The first `fatal error:` line, which is how a crashed linker reports.
    first_fatal: Option<String>,
    /// The crate and target of the last build cargo gave up on, as in
    /// `bullseye (test "controls")`.
    uncompiled: Option<String>,
}

/// One line describing why `step` failed with `status`, from its `output`.
pub fn failure_line(step: &str, status: &str, output: &str) -> String {
    let findings = scan(output);
    let mut line = format!("step `{step}` failed ({status})");
    if !findings.failed_tests.is_empty() {
        let _ = write!(
            line,
            "; failing tests: {}",
            findings.failed_tests.join(", ")
        );
    }
    if let Some(panic) = &findings.first_panic {
        let _ = write!(line, "; first panic: {panic}");
    }
    if findings.failed_tests.is_empty() && findings.first_panic.is_none() {
        let error = findings.first_fatal.or(findings.first_error);
        match (&findings.uncompiled, error) {
            (Some(target), Some(error)) => {
                let _ = write!(line, "; could not compile {target}: {error}");
            }
            (Some(target), None) => {
                let _ = write!(line, "; could not compile {target}");
            }
            (None, Some(error)) => {
                let _ = write!(line, "; {error}");
            }
            (None, None) => {}
        }
    }
    line
}

/// `text` without terminal colour and cursor escapes, which `CARGO_TERM_COLOR=always`
/// puts in CI logs.
fn strip_ansi(text: &str) -> String {
    let mut plain = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c != '\x1b' {
            plain.push(c);
        } else if chars.as_str().starts_with('[') {
            chars.next();
            for c in chars.by_ref() {
                if ('@'..='~').contains(&c) {
                    break;
                }
            }
        }
    }
    plain
}

fn scan(output: &str) -> Findings {
    let mut findings = Findings::default();
    let mut binary = None;
    let output = strip_ansi(output);
    let mut lines = output.lines();
    while let Some(line) = lines.next() {
        if let Some(path) = line.trim_start().strip_prefix("Running ") {
            binary = path.split_whitespace().next().map(str::to_owned);
        } else if line.trim_start().starts_with("Doc-tests ") {
            binary = Some("doc-tests".to_owned());
        } else if let Some(name) = line
            .strip_prefix("test ")
            .and_then(|rest| rest.strip_suffix(" ... FAILED"))
        {
            findings.failed_tests.push(match &binary {
                Some(binary) => format!("{name} ({binary})"),
                None => name.to_owned(),
            });
        } else if let Some((_, location)) = line.split_once(" panicked at ") {
            if findings.first_panic.is_none() {
                let message = match location.strip_suffix(':') {
                    Some(location) => {
                        let message = lines.next().unwrap_or_default().trim();
                        format!("{location}: {message}")
                    }
                    None => location.to_owned(),
                };
                findings.first_panic = Some(clip(&message));
            }
        } else if let Some(rest) = line.strip_prefix("error: could not compile ") {
            let target = rest.split(" due to ").next().unwrap_or(rest);
            findings.uncompiled = Some(target.replace('`', ""));
        } else if findings.first_fatal.is_none() && line.trim_start().contains("fatal error:") {
            findings.first_fatal = Some(clip(line.trim()));
        } else if findings.first_error.is_none()
            && (line.starts_with("error") || line.starts_with("Diff in "))
            && !line.starts_with("error: test failed")
            && !line.starts_with("error: process didn't exit")
        {
            findings.first_error = Some(clip(line.trim()));
        }
    }
    findings
}

fn clip(line: &str) -> String {
    match line.char_indices().nth(MAX_LINE) {
        Some((end, _)) => format!("{}...", &line[..end]),
        None => line.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_FAILURE: &str = "\
   Compiling bullseye v0.1.0 (/work/bullseye)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 41.20s
     Running unittests src/lib.rs (target/debug/deps/bullseye-0123456789abcdef)

running 2 tests
test measure::tests::min_content_is_widest_word ... ok
test measure::tests::definite_width_wraps ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/layout.rs (target/debug/deps/layout-fedcba9876543210)

running 3 tests
test kaolin_hello_world ... ok
test nowrap_text_keeps_one_line_in_a_narrow_box ... FAILED
test application_leaves_are_measured_with_borrowed_data ... FAILED

failures:

---- nowrap_text_keeps_one_line_in_a_narrow_box stdout ----

thread 'nowrap_text_keeps_one_line_in_a_narrow_box' (4242) panicked at tests/layout.rs:802:5:
assertion `left == right` failed
  left: 20.0
 right: 40.0

---- application_leaves_are_measured_with_borrowed_data stdout ----

thread 'application_leaves_are_measured_with_borrowed_data' (4243) panicked at tests/layout.rs:830:9:
called `Option::unwrap()` on a `None` value


failures:
    nowrap_text_keeps_one_line_in_a_narrow_box
    application_leaves_are_measured_with_borrowed_data

test result: FAILED. 1 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s

error: test failed, to rerun pass `--test layout`
";

    #[test]
    fn a_test_failure_names_every_failing_test_and_the_first_panic() {
        assert_eq!(
            failure_line("test", "exit status: 101", TEST_FAILURE),
            "step `test` failed (exit status: 101); failing tests: \
             nowrap_text_keeps_one_line_in_a_narrow_box (tests/layout.rs), \
             application_leaves_are_measured_with_borrowed_data (tests/layout.rs); \
             first panic: tests/layout.rs:802:5: assertion `left == right` failed"
        );
    }

    #[test]
    fn a_panic_on_one_line_is_kept_whole() {
        let output = "thread 'main' panicked at 'index out of bounds', src/lib.rs:3:9\n";
        assert_eq!(
            failure_line("test", "exit status: 101", output),
            "step `test` failed (exit status: 101); \
             first panic: 'index out of bounds', src/lib.rs:3:9"
        );
    }

    #[test]
    fn a_failing_doctest_is_named_with_its_binary() {
        let output = "\
   Doc-tests bullseye

running 1 test
test src/lib.rs - (line 3) ... FAILED
";
        assert_eq!(
            failure_line("doctest", "exit status: 101", output),
            "step `doctest` failed (exit status: 101); \
             failing tests: src/lib.rs - (line 3) (doc-tests)"
        );
    }

    #[test]
    fn a_compile_error_is_quoted_instead() {
        let output = "\
error[E0599]: no method named `p_px` found for struct `Tw` in the current scope
  --> tests/debug.rs:74:52

error: could not compile `bullseye` (test \"debug\") due to 1 previous error
";
        assert_eq!(
            failure_line("clippy", "exit status: 101", output),
            "step `clippy` failed (exit status: 101); \
             could not compile bullseye (test \"debug\"): \
             error[E0599]: no method named `p_px` found for struct `Tw` in the current scope"
        );
    }

    #[test]
    fn a_crashed_linker_is_named_with_the_target_it_was_linking() {
        let output = include_str!("summary_fixtures/linker_bus_error.log");
        assert!(
            output.contains('\x1b'),
            "the fixture must keep its ANSI codes"
        );
        assert_eq!(
            failure_line("test", "exit status: 101", output),
            "step `test` failed (exit status: 101); \
             could not compile bullseye (test \"controls\"): \
             collect2: fatal error: ld terminated with signal 7 [Bus error], core dumped"
        );
    }

    #[test]
    fn ansi_escapes_are_stripped_before_parsing() {
        assert_eq!(strip_ansi("\x1b[1m\x1b[91merror\x1b[0m: x"), "error: x");
        let output =
            "\x1b[1m\x1b[91merror\x1b[0m: could not compile `a` (lib) due to 1 previous error\n";
        assert_eq!(
            failure_line("c", "exit status: 101", output),
            "step `c` failed (exit status: 101); could not compile a (lib)"
        );
    }

    #[test]
    fn a_formatting_diff_is_quoted() {
        let output = "Diff in /work/bullseye/src/lib.rs:12:\n-old\n+new\n";
        assert_eq!(
            failure_line("fmt", "exit status: 1", output),
            "step `fmt` failed (exit status: 1); Diff in /work/bullseye/src/lib.rs:12:"
        );
    }

    #[test]
    fn a_step_that_says_nothing_still_has_its_status() {
        assert_eq!(
            failure_line("test", "signal: 11 (SIGSEGV)", "running 1 test\n"),
            "step `test` failed (signal: 11 (SIGSEGV))"
        );
    }

    #[test]
    fn long_messages_are_clipped() {
        let output = format!(
            "thread 't' (1) panicked at src/a.rs:1:1:\n{}\n",
            "x".repeat(500)
        );
        let line = failure_line("test", "exit status: 101", &output);
        assert!(line.ends_with("..."), "{line}");
        assert!(line.len() < 300, "{line}");
    }
}

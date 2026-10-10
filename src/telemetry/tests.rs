use std::{collections::HashSet, fs, process::Command};

use serde_json::Value;

#[test]
fn captures_stdout_and_overwrites_multithreaded_trace() -> Result<(), Box<dyn std::error::Error>> {
    let directory = std::env::temp_dir().join(format!("reng-trace-test-{}", std::process::id()));
    fs::create_dir(&directory)?;
    let trace_path = directory.join("trace.json");

    for run in ["first", "second", "debug", "error"] {
        fs::write(&trace_path, "old trace contents that must be overwritten")?;
        let mut command = Command::new(std::env::current_exe()?);
        command
            .args([
                "--exact",
                "telemetry::tests::capture_fixture",
                "--ignored",
                "--nocapture",
            ])
            .current_dir(&directory)
            .env_remove("RUST_LOG")
            .env("RENG_TRACE_TEST_RUN", run);
        if run == "debug" {
            command.env("RUST_LOG", "reng=debug");
        }
        let output = command.output()?;
        assert_eq!(output.status.success(), run != "error", "{output:?}");
        let stdout = String::from_utf8(output.stdout)?;
        assert!(stdout.contains("Main thread event"), "{stdout}");
        assert!(stdout.contains("Worker thread event"), "{stdout}");
        assert_eq!(stdout.contains("Debug event"), run == "debug", "{stdout}");

        let trace: Vec<Value> = serde_json::from_slice(&fs::read(&trace_path)?)?;
        for message in ["Main thread event", "Worker thread event"] {
            assert!(
                trace
                    .iter()
                    .any(|entry| entry["args"]["message"] == message)
            );
        }
        assert_eq!(
            trace
                .iter()
                .any(|entry| entry["args"]["message"] == "Debug event"),
            run == "debug"
        );
        let spans: Vec<_> = trace.iter().filter(|entry| entry["ph"] == "B").collect();
        assert!(spans.iter().any(|entry| entry["name"] == "main_work"));
        assert!(spans.iter().any(|entry| entry["name"] == "worker_work"));
        let threads: HashSet<_> = spans
            .iter()
            .map(|entry| entry["tid"].as_u64().unwrap())
            .collect();
        assert!(threads.len() >= 2, "{trace:?}");
        assert!(spans.iter().all(|entry| entry["args"]["run"] == run));
        for span in spans {
            assert!(
                trace
                    .iter()
                    .any(|entry| entry["ph"] == "E" && entry["tid"] == span["tid"])
            );
        }
    }

    fs::remove_dir_all(directory)?;
    Ok(())
}

// Run in a child process so the global subscriber and working directory stay isolated.
#[test]
#[ignore]
fn capture_fixture() -> Result<(), Box<dyn std::error::Error>> {
    let _guard = super::init()?;
    let run = std::env::var("RENG_TRACE_TEST_RUN")?;
    tracing::info_span!("main_work", run = %run).in_scope(|| {
        tracing::info!("Main thread event");
        tracing::debug!("Debug event");
    });
    let worker_run = run.clone();
    std::thread::spawn(move || {
        tracing::info_span!("worker_work", run = %worker_run).in_scope(|| {
            tracing::info!("Worker thread event");
        });
    })
    .join()
    .expect("test worker panicked");

    if run == "error" {
        return Err(std::io::Error::other("fixture returned error").into());
    }
    Ok(())
}

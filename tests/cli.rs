use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

struct Fixture(PathBuf);

impl Fixture {
    fn new(name: &str) -> Self {
        let time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        let path = std::env::temp_dir().join(format!(
            "cargo-depsize-{name}-{}-{}",
            std::process::id(),
            time.as_nanos()
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn write(&self, path: &str, content: &str) {
        let path = self.0.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    fn package(&self, dir: &str, name: &str, version: &str, extra: &str) -> usize {
        let manifest = format!(
            "[package]\nname = \"{name}\"\nversion = \"{version}\"\nedition = \"2024\"\n{extra}"
        );
        self.write(&format!("{dir}/Cargo.toml"), &manifest);
        self.write(&format!("{dir}/src/lib.rs"), "");
        manifest.len()
    }

    fn command(&self, directory: &str) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_cargo-depsize"));
        command.current_dir(self.0.join(directory));
        command.arg("depsize").arg("--offline");
        command
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn report(command: &mut Command) -> String {
    let result = command.output().unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    String::from_utf8(result.stdout).unwrap()
}

fn rows(output: &str) -> Vec<(&str, usize)> {
    output
        .lines()
        .filter_map(|line| line.split_once(" : "))
        .map(|(name, size)| {
            let bytes = size.strip_suffix(" bytes").unwrap().parse().unwrap();
            (name.trim_end(), bytes)
        })
        .collect()
}

#[test]
fn reports_resolved_direct_versions_and_only_source_files() {
    let fixture = Fixture::new("direct");
    fixture.package(
        "app",
        "app",
        "0.1.0",
        r#"
[dependencies]
selected = { package = "shared", path = "../v1" }
bridge = { path = "../bridge" }
feature_only = { path = "../feature", optional = true }

[target.'cfg(target_os = "none")'.dependencies]
other_target = { path = "../platform" }

[build-dependencies]
build_only = { path = "../build" }

[dev-dependencies]
dev_only = { path = "../dev" }
"#,
    );
    let selected_size = fixture.package("v1", "shared", "1.0.0", "");
    fixture.package("v2", "shared", "2.0.0", "");
    let bridge_size = fixture.package(
        "bridge",
        "bridge",
        "0.1.0",
        "\n[dependencies]\nshared = { path = \"../v2\" }\n",
    );
    let optional_size = fixture.package("feature", "feature_only", "0.1.0", "");
    let target_size = fixture.package("platform", "other_target", "0.1.0", "");
    fixture.package("build", "build_only", "0.1.0", "");
    fixture.package("dev", "dev_only", "0.1.0", "");
    fixture.write("v1/.ignore", "excluded.txt\n");
    fixture.write("v1/.hidden", "hidden");
    fixture.write("v1/excluded.txt", "ignored");
    fixture.write("v1/target/compiled", "compiled output");
    fixture.write("v1/custom-output/compiled", "configured compiled output");
    fixture.write("v1/src/payload.txt", "abc");
    #[cfg(unix)]
    std::os::unix::fs::symlink("payload.txt", fixture.0.join("v1/src/link")).unwrap();

    let output = report(
        fixture
            .command("app")
            .env("CARGO_TARGET_DIR", fixture.0.join("v1/custom-output")),
    );
    let selected_size = selected_size + 3;
    let entries = rows(&output);
    assert_eq!(entries.len(), 4, "{output}");
    assert!(entries.contains(&("shared (v1.0.0)", selected_size)));
    assert!(entries.contains(&("bridge (v0.1.0)", bridge_size)));
    assert!(entries.contains(&("feature_only (v0.1.0)", optional_size)));
    assert!(entries.contains(&("other_target (v0.1.0)", target_size)));
    let total = selected_size + bridge_size + optional_size + target_size;
    assert!(output.ends_with(&format!("> Total size: {total} bytes\n")));
}

#[test]
fn virtual_workspace_uses_default_members_and_deduplicates() {
    let fixture = Fixture::new("workspace");
    fixture.write(
        "ws/Cargo.toml",
        "[workspace]\nmembers = [\"a\", \"b\", \"c\"]\ndefault-members = [\"a\", \"b\"]\nresolver = \"3\"\n",
    );
    let first = "\n[dependencies]\ncommon = { path = \"../../common\" }\n";
    fixture.package("ws/a", "member_a", "0.1.0", first);
    fixture.package(
        "ws/b",
        "member_b",
        "0.1.0",
        &format!("{first}extra = {{ path = \"../../extra\" }}\n"),
    );
    fixture.package(
        "ws/c",
        "member_c",
        "0.1.0",
        "\n[dependencies]\nunused = { path = \"../../unused\" }\n",
    );
    let common_size = fixture.package("common", "common", "0.1.0", "");
    let extra_size = fixture.package("extra", "extra", "0.1.0", "");
    fixture.package("unused", "unused", "0.1.0", "");

    let output = report(&mut fixture.command("ws"));
    assert_eq!(
        rows(&output),
        [
            ("extra (v0.1.0)", extra_size),
            ("common (v0.1.0)", common_size),
        ],
    );
    assert!(output.ends_with(&format!(
        "> Total size: {} bytes\n",
        common_size + extra_size
    )));

    let member = report(fixture.command("ws/a").arg("--locked"));
    assert_eq!(rows(&member), [("common (v0.1.0)", common_size)],);
    let explicit =
        report(
            fixture
                .command(".")
                .args(["--manifest-path", "ws/a/Cargo.toml", "--locked"]),
        );
    assert_eq!(member, explicit);
}

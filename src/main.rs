use anyhow::{Context, Result, bail};
use cargo_metadata::{CargoOpt, DependencyKind, Metadata, MetadataCommand};
use std::collections::HashSet;
use std::env;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const HELP: &str = "Report the on-disk source size of direct Cargo dependencies.

Usage: cargo depsize [OPTIONS]

Options:
      --manifest-path <PATH>  Read the given Cargo.toml
      --locked                Require an up-to-date Cargo.lock
      --offline               Use cached Cargo dependencies only
  -h, --help                  Print help
  -V, --version               Print version";

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("Error: {err:#}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<()> {
    let mut args = env::args_os().skip(1).peekable();
    if args.peek().is_some_and(|arg| arg == "depsize") {
        args.next();
    }

    let mut command = MetadataCommand::new();
    let mut cargo_options = Vec::new();
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("-h" | "--help") => {
                println!("{HELP}");
                return Ok(());
            }
            Some("-V" | "--version") => {
                println!("cargo-depsize {}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            Some("--manifest-path") => {
                let path = args.next().context("--manifest-path requires a path")?;
                command.manifest_path(PathBuf::from(path));
            }
            Some("--locked") => cargo_options.push("--locked".to_owned()),
            Some("--offline") => cargo_options.push("--offline".to_owned()),
            _ => bail!("unknown argument {arg:?}. See `cargo depsize --help`"),
        }
    }

    let metadata = command
        .features(CargoOpt::AllFeatures)
        .other_options(cargo_options)
        .exec()
        .context("failed to resolve dependencies with cargo metadata")?;
    print_report(&metadata)
}

fn print_report(metadata: &Metadata) -> Result<()> {
    let resolve = metadata
        .resolve
        .as_ref()
        .context("Cargo did not return a dependency graph")?;
    let selected_members = if let Some(root) = &resolve.root {
        std::slice::from_ref(root)
    } else if metadata.workspace_default_members.is_available() {
        &metadata.workspace_default_members[..]
    } else {
        &metadata.workspace_members
    };
    let selected_members: HashSet<_> = selected_members.iter().collect();
    let direct_dependencies: HashSet<_> = resolve
        .nodes
        .iter()
        .filter(|node| selected_members.contains(&node.id))
        .flat_map(|node| &node.deps)
        .filter(|dep| {
            dep.dep_kinds
                .iter()
                .any(|kind| kind.kind == DependencyKind::Normal)
        })
        .map(|dep| &dep.pkg)
        .collect();

    let mut rows = Vec::with_capacity(direct_dependencies.len());
    let mut total = 0_u64;
    for package in &metadata.packages {
        if !direct_dependencies.contains(&package.id) {
            continue;
        }
        let root = package
            .manifest_path
            .parent()
            .with_context(|| format!("manifest for {} has no parent directory", package.id))?;
        let size = package_size(
            root.as_std_path(),
            metadata.target_directory.as_std_path(),
            metadata
                .build_directory
                .as_ref()
                .map(|path| path.as_std_path()),
        )
        .with_context(|| format!("failed to measure {}", package.id))?;

        total = total.checked_add(size).context("total size exceeds u64")?;
        rows.push((size, format!("{} (v{})", package.name, package.version)));
    }

    rows.sort_unstable();
    for (size, name) in rows {
        println!("{name:<25} : {}", format_size(size));
    }
    println!("> Total size: {}", format_size(total));
    Ok(())
}

fn package_size(root: &Path, target_dir: &Path, build_dir: Option<&Path>) -> Result<u64> {
    let local_target = root.join("target");
    let target_dir = target_dir.to_owned();
    let build_dir = build_dir.map(Path::to_owned);
    let walker = ignore::WalkBuilder::new(root)
        .filter_entry(move |entry| {
            let path = entry.path();
            entry.depth() == 0
                || !entry.file_type().is_some_and(|kind| kind.is_dir())
                || (path != local_target
                    && path != target_dir
                    && Some(path) != build_dir.as_deref())
        })
        .build();

    let mut total = 0_u64;
    for entry in walker {
        let entry = entry.context("failed to walk package files")?;
        if entry.file_type().is_some_and(|kind| kind.is_file()) {
            let path = entry.path();
            let metadata = std::fs::symlink_metadata(path)
                .with_context(|| format!("failed to read metadata for {}", path.display()))?;
            if metadata.is_file() {
                total = total
                    .checked_add(metadata.len())
                    .context("package size exceeds u64")?;
            }
        }
    }
    Ok(total)
}

fn format_size(size: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if size >= GB {
        format!("{:.2}GB ({size} bytes)", size as f64 / GB as f64)
    } else if size >= MB {
        format!("{:.2}MB ({size} bytes)", size as f64 / MB as f64)
    } else if size >= KB {
        format!("{:.2}KB ({size} bytes)", size as f64 / KB as f64)
    } else {
        format!("{size} bytes")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(1024), "1.00KB (1024 bytes)");
        assert_eq!(format_size(1048576), "1.00MB (1048576 bytes)");
        assert_eq!(format_size(1073741824), "1.00GB (1073741824 bytes)");
        assert_eq!(format_size(100), "100 bytes");
    }

    #[test]
    fn inaccessible_package_root_returns_an_error() {
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
        let missing_directory = manifest.join("not-a-directory");
        let error = package_size(&missing_directory, Path::new("target"), None).unwrap_err();

        assert!(error.to_string().contains("failed to walk package files"));
    }
}

use anyhow::{Context, Result};
use cargo::core::compiler::RustcTargetData;
use cargo::core::dependency::DepKind;
use cargo::core::resolver::features::{CliFeatures, ForceAllTargets, HasDevUnits};
use cargo::core::PackageId;
use cargo::core::Workspace;
use cargo::util::important_paths::find_root_manifest_for_wd;
use cargo::GlobalContext;
use std::collections::{HashMap, HashSet};
use std::env;
use std::process;

fn main() {
    let result = run();
    if let Err(err) = result {
        eprintln!("Error: {:?}", err);
        process::exit(1);
    }
}

fn run() -> Result<()> {
    let config = GlobalContext::default()?;

    // Locate the Cargo.toml
    let manifest_path = find_root_manifest_for_wd(&env::current_dir()?)?;

    // Create a workspace from the Cargo.toml
    let workspace = Workspace::new(&manifest_path, &config)?;

    // Calculate and display the total size of each dependency
    calculate_and_display_depsize(&workspace)?;

    Ok(())
}

/// Formats a size value in bytes as a human-readable string with units of KB, MB, or GB.
///
/// # Arguments
///
/// * `size` - The size value in bytes to format.
///
/// # Returns
///
/// Returns a `String` containing the formatted size value with units and byte count.
///
/// # Example
///
/// ```
/// assert_eq!(format_size(1024), "1.00KB (1024 bytes)");
/// assert_eq!(format_size(1048576), "1.00MB (1048576 bytes)");
/// assert_eq!(format_size(1073741824), "1.00GB (1073741824 bytes)");
/// assert_eq!(format_size(100), "100 bytes");
/// ```
fn format_size(size: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if size >= GB {
        format!("{:.2}GB ({:?} bytes)", size as f64 / GB as f64, size)
    } else if size >= MB {
        format!("{:.2}MB ({:?} bytes)", size as f64 / MB as f64, size)
    } else if size >= KB {
        format!("{:.2}KB ({:?} bytes)", size as f64 / KB as f64, size)
    } else {
        format!("{:?} bytes", size)
    }
}

/// Displays the size of each reported dependency and the total size.
fn calculate_and_display_depsize(workspace: &Workspace<'_>) -> Result<()> {
    // Obtain dependency graph
    let mut target_data = RustcTargetData::new(workspace, &[])?;
    let cli_features = CliFeatures::new_all(true);
    let has_dev_units = HasDevUnits::Yes;
    let force_all_targets = ForceAllTargets::Yes;

    let workspace_resolve = cargo::ops::resolve_ws_with_opts(
        workspace,
        &mut target_data,
        &[], // requested_targets
        &cli_features,
        &[], // specs
        has_dev_units,
        force_all_targets,
        false,
    )?;

    let mut package_sizes = HashMap::<PackageId, u64>::new();
    for package in workspace_resolve.pkg_set.packages() {
        let id = package.package_id();
        let size = calculate_package_size(package.root())
            .with_context(|| format!("Failed to calculate size for {id}"))?;
        package_sizes.insert(id, size);
    }

    let root_package = workspace.current()?;
    let root_deps = root_package
        .dependencies()
        .iter()
        .filter(|dep| dep.kind() == DepKind::Normal);

    // Identify the latest versions of each package among root dependencies
    // Collecting unique names of root dependencies
    let dep_names: HashSet<String> = root_deps
        .map(|dep| dep.package_name().to_string())
        .collect();

    // Resolving each dependency name to its latest version
    let latest_versions: HashSet<PackageId> = dep_names
        .into_iter()
        .filter_map(|name| {
            workspace_resolve
                .pkg_set
                .packages()
                .filter(|pkg| pkg.name() == name.as_str())
                .max_by_key(|pkg| pkg.version())
                .map(|pkg| pkg.package_id().clone())
        })
        .collect();

    let mut sum: u64 = 0;
    let mut package_infos = Vec::new();

    // Loop over the latest_versions HashSet
    for package_id in latest_versions.iter() {
        // Check if the package_id is in the package_sizes HashMap
        if let Some(&size) = package_sizes.get(package_id) {
            // Get the package from the package set to print its name and version
            if let Ok(package) = workspace_resolve.pkg_set.get_one(*package_id) {
                let name_ver = format!("{} (v{})", package.name(), package.version());
                package_infos.push((name_ver, size));
                sum += size;
            }
        }
    }

    // Sort the vector by size (second element of the tuple)
    package_infos.sort_by_key(|k| k.1);

    // Now iterate over the sorted vector (asc order)
    for (name_ver, size) in package_infos {
        println!("{: <25} : {}", name_ver, format_size(size));
    }

    println!("> Total size: {}", format_size(sum));

    Ok(())
}

fn calculate_package_size(package_path: &std::path::Path) -> Result<u64> {
    let walker = ignore::WalkBuilder::new(package_path).build();
    let mut total_size = 0;

    for entry in walker {
        match entry {
            Ok(entry) => {
                if entry.file_type().is_some_and(|kind| kind.is_file()) {
                    let path = entry.path();
                    let metadata = std::fs::metadata(path).with_context(|| {
                        format!("Failed to read metadata for {}", path.display())
                    })?;
                    total_size += metadata.len();
                }
            }
            Err(err) => eprintln!("Error: {}", err),
        }
    }

    Ok(total_size)
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
}

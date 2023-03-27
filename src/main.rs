use anyhow::Result;
use cargo::core::compiler::RustcTargetData;
use cargo::core::dependency::DepKind;
use cargo::core::resolver::features::{CliFeatures, ForceAllTargets, HasDevUnits};
use cargo::core::PackageId;
use cargo::core::Workspace;
use cargo::util::important_paths::find_root_manifest_for_wd;
use cargo::Config;
use std::collections::HashMap;
use std::env;
use std::process;
use tokio::fs;

#[tokio::main]
async fn main() {
    let result = run().await;
    if let Err(err) = result {
        eprintln!("Error: {:?}", err);
        process::exit(1);
    }
}

async fn run() -> Result<()> {
    let config = Config::default()?;

    // Locate the Cargo.toml
    let manifest_path = find_root_manifest_for_wd(&env::current_dir()?)?;

    // Create a workspace from the Cargo.toml
    let workspace = Workspace::new(&manifest_path, &config)?;

    // Calculate and display the total size of each dependency
    calculate_and_display_depsize(&workspace).await?;

    Ok(())
}

// choosing a specific version of the package when there are multiple versions available.
// fn select_latest_version(package_ids: &[PackageId]) -> &PackageId {
//     package_ids
//         .iter()
//         .max_by_key(|package_id| package_id.version())
//         .unwrap()
// }

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

/// Asynchronously calculates and displays the size of each dependency package
/// for the current workspace, as well as the total size of all dependencies.
///
/// # Arguments
///
/// * `workspace` - A reference to the `Workspace` object representing the current Rust workspace.
///
/// # Returns
///
/// Returns a `Result` indicating whether the operation was successful or not.
///
/// # Errors
///
/// Returns an error if any of the necessary dependencies cannot be resolved or if
/// there is an error while calculating the size of a package.
///
/// # Example
///
/// ```
/// use cargo::core::Workspace;
///
/// async fn example(workspace: &Workspace<'_>) {
///     if let Err(e) = calculate_and_display_depsize(workspace).await {
///         eprintln!("Error: {}", e);
///     }
/// }
/// ```
async fn calculate_and_display_depsize(workspace: &Workspace<'_>) -> Result<()> {
    // Obtain dependency graph
    // let requested_targets: Vec<CompileKind> = vec![];
    let target_data = RustcTargetData::new(workspace, &[])?;
    let cli_features = CliFeatures::new_all(true);
    //let specs: Vec<cargo::core::PackageIdSpec> = vec![];
    let has_dev_units = HasDevUnits::Yes;
    let force_all_targets = ForceAllTargets::Yes;

    let workspace_resolve = cargo::ops::resolve_ws_with_opts(
        workspace,
        &target_data,
        &[], // requested_targets
        &cli_features,
        &[], // specs
        has_dev_units,
        force_all_targets,
    )?;

    let packages = workspace_resolve.pkg_set.packages();
    // let resolve = workspace_resolve.workspace_resolve;
    let mut package_sizes = HashMap::<PackageId, u64>::new();

    for package in packages.into_iter() {
        let size = calculate_package_size(package).await?;
        package_sizes.insert(package.package_id().clone(), size);
    }

    let root_package = workspace.current()?;
    let root_deps = root_package
        .dependencies()
        .iter()
        .filter(|dep| dep.kind() == DepKind::Normal);

    let package_set = &workspace_resolve.pkg_set;

    let mut sum = 0;
    for dep in root_deps {
        let package_name = dep.package_name();
        let mut latest_package_id: Option<PackageId> = None;

        for package_id in package_set
            .package_ids()
            .filter(|id| id.name() == package_name)
        {
            latest_package_id = match latest_package_id {
                Some(current_latest) => Some(if package_id.version() > current_latest.version() {
                    package_id
                } else {
                    current_latest
                }),
                None => Some(package_id),
            };
        }

        let package_id = latest_package_id.unwrap();
        let package = package_set.get_one(package_id)?;
        let size = calculate_package_size(package).await?;
        sum += size;

        let name_ver = format!("{} (v{})", dep.name_in_toml(), package_id.version());
        println!("{: <25} : {}", name_ver, format_size(size));
    }

    println!("> Total size: {}", format_size(sum));

    Ok(())
}

async fn calculate_package_size(package: &cargo::core::Package) -> Result<u64> {
    let package_path = package.root();
    let walker = ignore::WalkBuilder::new(package_path).build();
    let mut total_size = 0;

    for entry in walker {
        match entry {
            Ok(entry) => {
                if entry.file_type().unwrap().is_file() {
                    let metadata = fs::metadata(entry.path()).await?;
                    total_size += metadata.len();
                }
            }
            Err(err) => eprintln!("Error: {}", err),
        }
    }

    Ok(total_size)
}

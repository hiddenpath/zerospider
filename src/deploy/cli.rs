//! CLI handlers for deploy commands.

use crate::config::Config;
use crate::deploy::remote::{DeploymentConfig, DeploymentMode, DeploymentStatus, RemoteDeployer};
use anyhow::{bail, Context, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::info;

/// Handle deploy subcommands.
pub async fn handle_command(
    deploy_command: crate::deploy::DeployCommands,
    config: &Config,
) -> Result<()> {
    let deploy_config = crate::config::DeployConfig::default();
    let targets = load_deploy_config(config, &deploy_config)?;

    match deploy_command {
        crate::deploy::DeployCommands::Deploy { server } => {
            handle_deploy(&server, &targets, &deploy_config).await
        }
        crate::deploy::DeployCommands::Status { server } => handle_status(&server, &targets).await,
        crate::deploy::DeployCommands::HealthCheck { server } => {
            handle_health_check(&server, &targets).await
        }
        crate::deploy::DeployCommands::List => handle_list(&targets),
        crate::deploy::DeployCommands::Rollback { server } => {
            handle_rollback(&server, &targets).await
        }
        crate::deploy::DeployCommands::Update { server, version } => {
            handle_update(&server, version, &targets, &deploy_config).await
        }
        crate::deploy::DeployCommands::SyncConfig { server } => {
            handle_sync_config(&server, config).await
        }
    }
}

/// Load deploy configuration from config.
fn load_deploy_config(
    config: &Config,
    _deploy_cfg: &crate::config::DeployConfig,
) -> Result<Vec<crate::deploy::remote::DeploymentTarget>> {
    if config.deploy.servers.is_empty() {
        bail!(
            "No deployment targets configured. \
             Add targets to your config.toml under [deploy.servers]"
        );
    }

    // Convert from config types to deployment runtime types
    let targets: Result<Vec<crate::deploy::remote::DeploymentTarget>> = config
        .deploy
        .servers
        .iter()
        .map(|cfg| convert_target_config(cfg))
        .collect();
    targets
}

/// Convert DeploymentTargetConfig to DeploymentTarget.
fn convert_target_config(
    cfg: &crate::config::DeploymentTargetConfig,
) -> Result<crate::deploy::remote::DeploymentTarget> {
    // Convert labels from Vec<String> to HashMap<String, String>
    let labels: HashMap<String, String> = cfg
        .labels
        .iter()
        .filter_map(|label| {
            // Parse key:value format
            if let Some((key, value)) = label.split_once(':') {
                Some((key.to_string(), value.to_string()))
            } else if let Some((key, value)) = label.split_once('=') {
                Some((key.to_string(), value.to_string()))
            } else {
                // If no separator, use the label as both key and value
                Some((label.clone(), label.clone()))
            }
        })
        .collect();

    // Convert ssh_key string to path
    let ssh_key_path = cfg.ssh_key.as_ref().map(|s| PathBuf::from(s));

    Ok(crate::deploy::remote::DeploymentTarget {
        id: cfg.id.clone(),
        host: cfg.host.clone(),
        port: cfg.port,
        user: cfg.user.clone(),
        ssh_key_path,
        labels,
    })
}

/// Parse deployment mode from config.
fn parse_deployment_mode(mode_str: &str) -> DeploymentMode {
    match mode_str.to_lowercase().as_str() {
        "docker" => DeploymentMode::Docker,
        "systemd" => DeploymentMode::Systemd,
        _ => DeploymentMode::Direct,
    }
}

/// Create deployment config from settings.
fn create_deployment_config(
    settings: &crate::config::DeploymentSettingsConfig,
    version: &str,
) -> DeploymentConfig {
    // Auto-detect local binary path
    let local_binary = std::env::current_exe()
        .and_then(|p| p.canonicalize())
        .unwrap_or_else(|_| PathBuf::from("./target/release/zerospider"));
    
    DeploymentConfig {
        name: "zerospider".to_string(),
        version: version.to_string(),
        local_binary,
        binary_path: PathBuf::from(&settings.binary_path),
        config_path: settings.config_path.as_ref().map(|p| PathBuf::from(p)),
        env_vars: HashMap::new(),
        working_dir: PathBuf::from(&settings.working_dir),
        auto_start: settings.auto_start,
        health_check_interval: std::time::Duration::from_secs(settings.health_check_interval_secs),
        restart_on_failure: settings.restart_on_failure,
        max_restarts: settings.max_restarts,
        use_sudo: settings.use_sudo,
    }
}

/// Handle deploy command.
async fn handle_deploy(
    server_id: &str,
    targets: &[crate::deploy::remote::DeploymentTarget],
    deploy_config: &crate::config::DeployConfig,
) -> Result<()> {
    let target = find_target(server_id, targets)?;

    info!("Starting deployment to {} ({})...", server_id, target.host);

    let mode = parse_deployment_mode(&deploy_config.settings.mode);
    let mut deployer = RemoteDeployer::new(mode);
    deployer.register_target(target.clone());

    let dep_config = create_deployment_config(&deploy_config.settings, "latest");
    deployer.set_config(dep_config);

    deployer
        .deploy(server_id, "zeroclaw")
        .await
        .context("Deployment failed")?;

    println!("✅ Deployed successfully to {}", server_id);

    Ok(())
}

/// Handle status command.
async fn handle_status(
    server_id: &str,
    targets: &[crate::deploy::remote::DeploymentTarget],
) -> Result<()> {
    let target = find_target(server_id, targets)?;

    info!("Checking deployment status for {}...", server_id);

    let mut deployer = RemoteDeployer::new(DeploymentMode::Direct);
    deployer.register_target(target.clone());

    let status = deployer
        .get_status(server_id)
        .context("Failed to get status")?;

    println!("📊 Deployment Status for {}", server_id);
    print_status(status);

    Ok(())
}

/// Print deployment status.
fn print_status(status: &DeploymentStatus) {
    println!("   Status: {:?}", status.target_id);
    println!("   Deployed: {}", status.deployed);
    if let Some(version) = &status.version {
        println!("   Version: {}", version);
    }
    println!("   Running: {}", status.running);
    if status.healthy() {
        println!("   Health: ✅ Healthy");
    } else {
        println!("   Health: ❌ Unhealthy");
    }
}

/// Handle health-check command.
async fn handle_health_check(
    server_id: &str,
    targets: &[crate::deploy::remote::DeploymentTarget],
) -> Result<()> {
    let target = find_target(server_id, targets)?;

    info!("Running health check for {}...", server_id);

    let mut deployer = RemoteDeployer::new(DeploymentMode::Direct);
    deployer.register_target(target.clone());

    let healthy = deployer
        .health_check(server_id)
        .await
        .context("Health check failed")?;

    if healthy {
        println!("✅ Health check passed for {}", server_id);
    } else {
        bail!("❌ Health check failed for {}", server_id);
    }

    Ok(())
}

/// Handle list command.
fn handle_list(targets: &[crate::deploy::remote::DeploymentTarget]) -> Result<()> {
    if targets.is_empty() {
        println!("No deployment targets configured.");
        return Ok(());
    }

    println!("🌐 Deployment Targets ({} total):", targets.len());
    println!();
    for target in targets {
        println!("  ID: {}", target.id);
        println!("  Host: {}:{}", target.host, target.port);
        println!("  User: {}", target.user);
        if let Some(ssh_key) = &target.ssh_key_path {
            println!("  SSH Key: {}", ssh_key.display());
        }
        if !target.labels.is_empty() {
            println!("  Labels:");
            for (key, value) in &target.labels {
                println!("    {} = {}", key, value);
            }
        }
        println!();
    }

    Ok(())
}

/// Handle rollback command.
async fn handle_rollback(
    server_id: &str,
    targets: &[crate::deploy::remote::DeploymentTarget],
) -> Result<()> {
    let target = find_target(server_id, targets)?;

    info!("Rolling back deployment on {}...", server_id);

    let mut deployer = RemoteDeployer::new(DeploymentMode::Direct);
    deployer.register_target(target.clone());

    deployer
        .rollback(server_id)
        .await
        .context("Rollback failed")?;

    println!("✅ Rolled back successfully on {}", server_id);

    Ok(())
}

/// Handle update command.
async fn handle_update(
    server_id: &str,
    version: Option<String>,
    targets: &[crate::deploy::remote::DeploymentTarget],
    deploy_config: &crate::config::DeployConfig,
) -> Result<()> {
    let target = find_target(server_id, targets)?;

    let version = version.unwrap_or_else(|| "latest".to_string());

    info!("Updating {} to version {}...", server_id, version);

    // For now, update is essentially the same as deploy
    // In a more sophisticated implementation, this would:
    // 1. Check current version
    // 2. Download new binary
    // 3. Perform rolling update

    println!(
        "ℹ️  Update will deploy version {} to {}",
        version, server_id
    );
    println!("   This is equivalent to a deploy operation with the specified version.");

    let mode = parse_deployment_mode(&deploy_config.settings.mode);
    let mut deployer = RemoteDeployer::new(mode);
    deployer.register_target(target.clone());

    let dep_config = create_deployment_config(&deploy_config.settings, &version);
    deployer.set_config(dep_config);

    deployer
        .deploy(server_id, "zeroclaw")
        .await
        .context("Update failed")?;

    println!("✅ Updated successfully to {} on {}", version, server_id);

    Ok(())
}

/// Handle sync-config command.
async fn handle_sync_config(_server_id: &str, _config: &Config) -> Result<()> {
    bail!(
        "sync-config is not yet implemented. \
         To update the remote configuration, manually edit the config file on the target server \
         or redeploy with the new configuration."
    );
}

/// Find a deployment target by ID.
fn find_target<'a>(
    server_id: &str,
    targets: &'a [crate::deploy::remote::DeploymentTarget],
) -> Result<&'a crate::deploy::remote::DeploymentTarget> {
    targets
        .iter()
        .find(|t| t.id == server_id)
        .ok_or_else(|| anyhow::anyhow!("Deployment target '{server_id}' not found"))
}

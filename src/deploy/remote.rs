//! Controlled remote deployment module.
//!
//! Provides secure remote server deployment capabilities:
//! - SSH-based deployment
//! - Configuration-driven setup
//! - Health monitoring
//! - Rollback support

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct DeploymentTarget {
    pub id: String,
    pub host: String,
    pub port: u16,
    pub user: String,
    pub ssh_key_path: Option<PathBuf>,
    pub labels: HashMap<String, String>,
}

impl DeploymentTarget {
    pub fn new(id: impl Into<String>, host: impl Into<String>, user: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            host: host.into(),
            port: 22,
            user: user.into(),
            ssh_key_path: None,
            labels: HashMap::new(),
        }
    }

    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    pub fn with_ssh_key(mut self, path: impl Into<PathBuf>) -> Self {
        self.ssh_key_path = Some(path.into());
        self
    }

    pub fn with_label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.labels.insert(key.into(), value.into());
        self
    }
}

#[derive(Debug, Clone)]
pub struct DeploymentConfig {
    pub name: String,
    pub version: String,
    pub binary_path: PathBuf,
    pub config_path: Option<PathBuf>,
    pub env_vars: HashMap<String, String>,
    pub working_dir: PathBuf,
    pub auto_start: bool,
    pub health_check_interval: Duration,
    pub restart_on_failure: bool,
    pub max_restarts: u32,
}

impl Default for DeploymentConfig {
    fn default() -> Self {
        Self {
            name: "zeroclaw".to_string(),
            version: "latest".to_string(),
            binary_path: PathBuf::from("/usr/local/bin/zeroclaw"),
            config_path: None,
            env_vars: HashMap::new(),
            working_dir: PathBuf::from("/opt/zeroclaw"),
            auto_start: true,
            health_check_interval: Duration::from_secs(30),
            restart_on_failure: true,
            max_restarts: 3,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DeploymentStatus {
    pub target_id: String,
    pub deployed: bool,
    pub version: Option<String>,
    pub running: bool,
    pub last_health_check: Option<std::time::Instant>,
    pub uptime: Option<Duration>,
    pub restart_count: u32,
}

impl DeploymentStatus {
    pub fn healthy(&self) -> bool {
        self.deployed && self.running
    }
}

#[derive(Debug, Clone)]
pub struct DeploymentStep {
    pub name: String,
    pub command: String,
    pub expected_exit_code: i32,
    pub timeout: Duration,
    pub rollback_command: Option<String>,
}

impl DeploymentStep {
    pub fn new(name: impl Into<String>, command: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            command: command.into(),
            expected_exit_code: 0,
            timeout: Duration::from_secs(60),
            rollback_command: None,
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn with_rollback(mut self, command: impl Into<String>) -> Self {
        self.rollback_command = Some(command.into());
        self
    }
}

#[derive(Debug, Clone)]
pub enum DeploymentMode {
    Direct,
    Docker,
    Systemd,
}

pub struct RemoteDeployer {
    targets: HashMap<String, DeploymentTarget>,
    configs: HashMap<String, DeploymentConfig>,
    statuses: HashMap<String, DeploymentStatus>,
    mode: DeploymentMode,
}

impl RemoteDeployer {
    pub fn new(mode: DeploymentMode) -> Self {
        Self {
            targets: HashMap::new(),
            configs: HashMap::new(),
            statuses: HashMap::new(),
            mode,
        }
    }

    pub fn register_target(&mut self, target: DeploymentTarget) {
        let id = target.id.clone();
        self.statuses.insert(
            id.clone(),
            DeploymentStatus {
                target_id: id.clone(),
                deployed: false,
                version: None,
                running: false,
                last_health_check: None,
                uptime: None,
                restart_count: 0,
            },
        );
        self.targets.insert(id, target);
    }

    pub fn set_config(&mut self, config: DeploymentConfig) {
        self.configs.insert(config.name.clone(), config);
    }

    pub async fn deploy(&mut self, target_id: &str, config_name: &str) -> anyhow::Result<()> {
        let target = self
            .targets
            .get(target_id)
            .ok_or_else(|| anyhow::anyhow!("Target not found: {}", target_id))?
            .clone();

        let config = self
            .configs
            .get(config_name)
            .ok_or_else(|| anyhow::anyhow!("Config not found: {}", config_name))?
            .clone();

        let steps = self.build_deployment_steps(&target, &config);

        for step in steps {
            match self.execute_step(&target, &step).await {
                Ok(_) => {
                    tracing::info!(step = %step.name, target = %target_id, "Step completed");
                }
                Err(e) => {
                    tracing::error!(step = %step.name, target = %target_id, error = %e, "Step failed");

                    if let Some(rollback) = &step.rollback_command {
                        tracing::info!(target = %target_id, "Executing rollback");
                        let _ = self.execute_raw(&target, rollback).await;
                    }

                    return Err(e);
                }
            }
        }

        if let Some(status) = self.statuses.get_mut(target_id) {
            status.deployed = true;
            status.version = Some(config.version.clone());
            status.running = config.auto_start;
        }

        Ok(())
    }

    fn build_deployment_steps(
        &self,
        target: &DeploymentTarget,
        config: &DeploymentConfig,
    ) -> Vec<DeploymentStep> {
        match self.mode {
            DeploymentMode::Direct => vec![
                DeploymentStep::new(
                    "create_dir",
                    format!("mkdir -p {}", config.working_dir.display()),
                )
                .with_rollback(format!("rm -rf {}", config.working_dir.display())),
                DeploymentStep::new(
                    "upload_binary",
                    format!(
                        "scp {} {}@{}:{}",
                        config.binary_path.display(),
                        target.user,
                        target.host,
                        config.binary_path.display()
                    ),
                ),
                DeploymentStep::new(
                    "make_executable",
                    format!("chmod +x {}", config.binary_path.display()),
                ),
                DeploymentStep::new(
                    "start_service",
                    format!(
                        "cd {} && {} &",
                        config.working_dir.display(),
                        config.binary_path.display()
                    ),
                ),
            ],
            DeploymentMode::Docker => vec![
                DeploymentStep::new(
                    "pull_image",
                    format!("docker pull zeroclaw:{}", config.version),
                ),
                DeploymentStep::new("stop_existing", "docker stop zeroclaw || true".to_string()),
                DeploymentStep::new("remove_existing", "docker rm zeroclaw || true".to_string()),
                DeploymentStep::new(
                    "run_container",
                    format!(
                        "docker run -d --name zeroclaw -p 8080:8080 zeroclaw:{}",
                        config.version
                    ),
                ),
            ],
            DeploymentMode::Systemd => vec![
                DeploymentStep::new(
                    "upload_binary",
                    format!(
                        "scp {} {}@{}:{}",
                        config.binary_path.display(),
                        target.user,
                        target.host,
                        config.binary_path.display()
                    ),
                ),
                DeploymentStep::new("install_service", "systemctl daemon-reload".to_string()),
                DeploymentStep::new("enable_service", "systemctl enable zeroclaw".to_string()),
                DeploymentStep::new("start_service", "systemctl start zeroclaw".to_string()),
            ],
        }
    }

    async fn execute_step(
        &self,
        target: &DeploymentTarget,
        step: &DeploymentStep,
    ) -> anyhow::Result<()> {
        self.execute_raw(target, &step.command).await
    }

    async fn execute_raw(&self, target: &DeploymentTarget, command: &str) -> anyhow::Result<()> {
        let ssh_target = format!("{}@{}:{}", target.user, target.host, target.port);

        tracing::info!(target = %ssh_target, command = %command, "Executing remote command");

        let output = tokio::time::timeout(
            Duration::from_secs(60),
            tokio::process::Command::new("ssh")
                .arg("-o")
                .arg("StrictHostKeyChecking=no")
                .arg("-p")
                .arg(target.port.to_string())
                .arg(format!("{}@{}", target.user, target.host))
                .arg(command)
                .output(),
        )
        .await
        .map_err(|e| anyhow::anyhow!("Command timeout: {}", e))?
        .map_err(|e| anyhow::anyhow!("SSH execution failed: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Command failed: {}", stderr);
        }

        Ok(())
    }

    pub async fn health_check(&mut self, target_id: &str) -> anyhow::Result<bool> {
        let target = self
            .targets
            .get(target_id)
            .ok_or_else(|| anyhow::anyhow!("Target not found: {}", target_id))?;

        let result = self
            .execute_raw(target, "pgrep -x zeroclaw > /dev/null")
            .await;

        if let Some(status) = self.statuses.get_mut(target_id) {
            status.running = result.is_ok();
            status.last_health_check = Some(std::time::Instant::now());
        }

        Ok(result.is_ok())
    }

    pub async fn rollback(&mut self, target_id: &str) -> anyhow::Result<()> {
        let target = self
            .targets
            .get(target_id)
            .ok_or_else(|| anyhow::anyhow!("Target not found: {}", target_id))?;

        let rollback_steps = match self.mode {
            DeploymentMode::Direct => vec!["pkill -x zeroclaw || true", "rm -rf /opt/zeroclaw"],
            DeploymentMode::Docker => {
                vec!["docker stop zeroclaw || true", "docker rm zeroclaw || true"]
            }
            DeploymentMode::Systemd => vec![
                "systemctl stop zeroclaw || true",
                "systemctl disable zeroclaw || true",
            ],
        };

        for step in rollback_steps {
            let _ = self.execute_raw(target, step).await;
        }

        if let Some(status) = self.statuses.get_mut(target_id) {
            status.deployed = false;
            status.running = false;
        }

        Ok(())
    }

    pub fn get_status(&self, target_id: &str) -> Option<&DeploymentStatus> {
        self.statuses.get(target_id)
    }

    pub fn list_targets(&self) -> Vec<&DeploymentTarget> {
        self.targets.values().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deployment_target_builder() {
        let target = DeploymentTarget::new("server-1", "192.168.1.100", "deploy")
            .with_port(2222)
            .with_ssh_key("~/.ssh/deploy_key")
            .with_label("env", "production");

        assert_eq!(target.id, "server-1");
        assert_eq!(target.port, 2222);
        assert_eq!(target.labels.get("env"), Some(&"production".to_string()));
    }

    #[test]
    fn test_deployment_config_default() {
        let config = DeploymentConfig::default();
        assert_eq!(config.name, "zeroclaw");
        assert!(config.auto_start);
        assert!(config.restart_on_failure);
    }

    #[test]
    fn test_deployment_step_builder() {
        let step = DeploymentStep::new("deploy", "cp binary /usr/bin/")
            .with_timeout(Duration::from_secs(120))
            .with_rollback("rm /usr/bin/binary");

        assert_eq!(step.name, "deploy");
        assert_eq!(step.timeout, Duration::from_secs(120));
        assert!(step.rollback_command.is_some());
    }
}

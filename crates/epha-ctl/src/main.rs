use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Parser)]
#[command(name = "epha-ctl", about = "Ephemera AI service manager")]
struct Cli {
    /// Config directory
    #[arg(long, env = "EPHA_CONFIG_DIR")]
    config_dir: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Show service status
    Status {
        /// Service name (omit for all)
        service: Option<String>,

        /// Tree view (dependency hierarchy)
        #[arg(long)]
        tree: bool,
    },
    /// Start a service
    Start {
        /// Service name, or --all
        #[arg(conflicts_with = "all")]
        service: Option<String>,

        /// Start all services
        #[arg(long)]
        all: bool,
    },
    /// Stop a service
    Stop {
        #[arg(conflicts_with = "all")]
        service: Option<String>,

        /// Stop all services
        #[arg(long)]
        all: bool,
    },
    /// Restart a service
    Restart {
        #[arg(conflicts_with = "all")]
        service: Option<String>,

        /// Restart all services
        #[arg(long)]
        all: bool,
    },
    /// List discovered services
    List,
}

#[derive(Debug, Clone)]
struct ServiceStatus {
    active_state: String,
    sub_state: String,
}

impl fmt::Display for ServiceStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:<10} {}", self.active_state, self.sub_state)
    }
}

/// A discovered service with its tier classification.
#[derive(Debug, Clone)]
struct Service {
    name: String,
    tier: Tier,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Tier {
    Agent,
    Core,
    Domain,
    Herald,
    Dependency,
}

impl fmt::Display for Tier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Tier::Agent => write!(f, "Agent"),
            Tier::Core => write!(f, "Core"),
            Tier::Domain => write!(f, "Domain"),
            Tier::Herald => write!(f, "Heralds"),
            Tier::Dependency => write!(f, "Dependencies"),
        }
    }
}

/// Tree node for dependency hierarchy.
struct TreeNode {
    name: String,
    status: Option<ServiceStatus>,
    children: Vec<TreeNode>,
}

impl TreeNode {
    fn print(&self, is_last: &[bool]) {
        let connector = if is_last.is_empty() {
            "".to_string()
        } else {
            let mut s = String::new();
            for (i, &last) in is_last.iter().enumerate() {
                if i < is_last.len() - 1 {
                    s.push_str(if last { "    " } else { "│   " });
                } else {
                    s.push_str(if last { "└── " } else { "├── " });
                }
            }
            s
        };

        let status_str = match &self.status {
            Some(s) => s.to_string(),
            None => "not found".to_string(),
        };

        println!(
            "{connector}{name:<22} {status_str}",
            connector = connector,
            name = self.name,
            status_str = status_str
        );

        for (i, child) in self.children.iter().enumerate() {
            let mut child_is_last = is_last.to_vec();
            child_is_last.push(i == self.children.len() - 1);
            child.print(&child_is_last);
        }
    }
}

// --- Loom config: { "mysql": { "url": "..." }, ... }
#[derive(Deserialize)]
struct LoomConfig {
    mysql: LoomMySql,
}

#[derive(Deserialize)]
struct LoomMySql {
    url: String,
}

// --- Atrium config: { "mysql_url": "...", ... }
#[derive(Deserialize)]
struct AtriumConfig {
    mysql_url: String,
}

fn default_config_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| {
            std::env::var("HOME")
                .map(|h| PathBuf::from(h).join(".local/share"))
                .unwrap_or_else(|_| PathBuf::from(".local/share"))
        })
        .join("ephemera")
}

/// Discover services by scanning config directory subdirectories.
fn discover_services(config_dir: &Path) -> Vec<Service> {
    let mut services = Vec::new();

    let entries = match std::fs::read_dir(config_dir) {
        Ok(e) => e,
        Err(e) => {
            eprintln!(
                "warning: cannot read config directory {}: {e}",
                config_dir.display()
            );
            return services;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };

        // Check if directory contains any .json file
        let has_json = std::fs::read_dir(&path).is_ok_and(|entries| {
            entries
                .flatten()
                .any(|e| e.path().extension().is_some_and(|ext| ext == "json"))
        });

        if !has_json {
            continue;
        }

        let tier = match name.as_str() {
            "epha-ai" => Tier::Agent,
            "agora" | "loom" => Tier::Core,
            "kairos" | "atrium" => Tier::Domain,
            "kairos-herald" | "atrium-herald" => Tier::Herald,
            _ => Tier::Dependency,
        };

        services.push(Service { name, tier });
    }

    services.sort_by(|a, b| (&a.tier, &a.name).cmp(&(&b.tier, &b.name)));
    services
}

/// Extract MySQL URL from a loom config file.
fn extract_loom_mysql_url(config_dir: &Path) -> Option<String> {
    let path = config_dir.join("loom").join("loom.json");
    let content = std::fs::read_to_string(&path).ok()?;
    let config: LoomConfig = serde_json::from_str(&content).ok()?;
    Some(config.mysql.url)
}

/// Extract MySQL URL from an atrium config file.
fn extract_atrium_mysql_url(config_dir: &Path) -> Option<String> {
    let path = config_dir.join("atrium").join("atrium.json");
    let content = std::fs::read_to_string(&path).ok()?;
    let config: AtriumConfig = serde_json::from_str(&content).ok()?;
    Some(config.mysql_url)
}

/// Check if a MySQL URL points to localhost by parsing its host.
fn is_localhost_mysql(url: &str) -> bool {
    let parsed = match url::Url::parse(url) {
        Ok(u) => u,
        Err(_) => return false,
    };
    matches!(parsed.host_str(), Some("localhost" | "127.0.0.1" | "::1"))
}

/// Discover MySQL services from loom/atrium configs.
fn discover_mysql_services(config_dir: &Path) -> Vec<Service> {
    let mut mysql_services = Vec::new();

    if let Some(url) = extract_loom_mysql_url(config_dir)
        && is_localhost_mysql(&url)
    {
        mysql_services.push(Service { name: "loom-mysql".to_string(), tier: Tier::Dependency });
    }

    if let Some(url) = extract_atrium_mysql_url(config_dir)
        && is_localhost_mysql(&url)
    {
        mysql_services.push(Service { name: "atrium-mysql".to_string(), tier: Tier::Dependency });
    }

    mysql_services
}

/// Query systemd unit status.
fn query_unit_status(unit: &str, user: bool) -> Option<ServiceStatus> {
    let mut cmd = Command::new("systemctl");
    if user {
        cmd.arg("--user");
    }
    cmd.args(["show", unit, "--property=ActiveState,SubState"]);

    let output = cmd.output().ok()?;
    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut active_state = String::new();
    let mut sub_state = String::new();

    for line in stdout.lines() {
        if let Some(value) = line.strip_prefix("ActiveState=") {
            active_state = value.to_string();
        } else if let Some(value) = line.strip_prefix("SubState=") {
            sub_state = value.to_string();
        }
    }

    if active_state.is_empty() {
        return None;
    }

    Some(ServiceStatus { active_state, sub_state })
}

/// Get status for a service, trying user-level first, then system-level for MySQL.
fn get_service_status(name: &str) -> Option<ServiceStatus> {
    let unit = format!("{}.service", name);

    // Try user-level first
    if let Some(status) = query_unit_status(&unit, true) {
        return Some(status);
    }

    // For MySQL services, also try system-level
    if name.ends_with("-mysql")
        && let Some(status) = query_unit_status("mysql.service", false)
    {
        return Some(status);
    }

    None
}

/// Build the dependency tree.
fn build_tree(services: &[Service], mysql_services: &[Service]) -> TreeNode {
    let service_map: HashMap<&str, &Service> = services
        .iter()
        .chain(mysql_services.iter())
        .map(|s| (s.name.as_str(), s))
        .collect();

    let mysql_set: std::collections::HashSet<&str> =
        mysql_services.iter().map(|s| s.name.as_str()).collect();

    let mut root_children: Vec<TreeNode> = Vec::new();

    // Core tier: agora, loom
    for core_name in &["agora", "loom"] {
        if service_map.contains_key(*core_name) {
            let mut children = Vec::new();
            if *core_name == "loom" {
                let mysql_name = "loom-mysql";
                if mysql_set.contains(mysql_name) {
                    children.push(TreeNode {
                        name: mysql_name.to_string(),
                        status: get_service_status(mysql_name),
                        children: Vec::new(),
                    });
                }
            }
            root_children.push(TreeNode {
                name: (*core_name).to_string(),
                status: get_service_status(core_name),
                children,
            });
        }
    }

    // Domain tier: atrium, kairos
    for domain_name in &["atrium", "kairos"] {
        if service_map.contains_key(*domain_name) {
            let mut children = Vec::new();
            if *domain_name == "atrium" {
                let herald_name = "atrium-herald";
                if service_map.contains_key(herald_name) {
                    children.push(TreeNode {
                        name: herald_name.to_string(),
                        status: get_service_status(herald_name),
                        children: Vec::new(),
                    });
                }
                let mysql_name = "atrium-mysql";
                if mysql_set.contains(mysql_name) {
                    children.push(TreeNode {
                        name: mysql_name.to_string(),
                        status: get_service_status(mysql_name),
                        children: Vec::new(),
                    });
                }
            } else {
                let herald_name = "kairos-herald";
                if service_map.contains_key(herald_name) {
                    children.push(TreeNode {
                        name: herald_name.to_string(),
                        status: get_service_status(herald_name),
                        children: Vec::new(),
                    });
                }
            }
            root_children.push(TreeNode {
                name: (*domain_name).to_string(),
                status: get_service_status(domain_name),
                children,
            });
        }
    }

    root_children.sort_by(|a, b| {
        let order = ["agora", "loom", "atrium", "kairos"];
        let a_idx = order.iter().position(|&n| n == a.name).unwrap_or(99);
        let b_idx = order.iter().position(|&n| n == b.name).unwrap_or(99);
        a_idx.cmp(&b_idx)
    });

    let has_epha_ai = service_map.contains_key("epha-ai");

    TreeNode {
        name: "epha-ai".to_string(),
        status: if has_epha_ai { get_service_status("epha-ai") } else { None },
        children: root_children,
    }
}

fn action_service(action: &str, name: &str, mysql_count: usize) -> Result<()> {
    // Try user-level first
    let unit = format!("{}.service", name);
    let mut cmd = Command::new("systemctl");
    cmd.arg("--user");
    cmd.args([action, &unit]);
    let output = cmd
        .output()
        .with_context(|| format!("Failed to run systemctl {action} {unit}"))?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);

    // For MySQL services, try system-level
    if name.ends_with("-mysql") {
        // Refuse to stop/restart shared system MySQL individually
        if (action == "stop" || action == "restart") && mysql_count > 1 {
            bail!(
                "Refusing to {action} {name}: system MySQL is shared by {mysql_count} services. Use --all to {action} all services together."
            );
        }
        let mut cmd = Command::new("systemctl");
        cmd.args([action, "mysql.service"]);
        let output = cmd
            .output()
            .with_context(|| format!("Failed to run systemctl {action} mysql.service"))?;
        if output.status.success() {
            return Ok(());
        }
        let sys_stderr = String::from_utf8_lossy(&output.stderr);
        bail!("Failed to {action} {name}: {sys_stderr}");
    }

    bail!("Failed to {action} {name}: {stderr}");
}

/// Collect all service names from discovered services.
fn all_service_names<'a>(services: &'a [Service], mysql_services: &'a [Service]) -> Vec<&'a str> {
    services
        .iter()
        .chain(mysql_services.iter())
        .map(|s| s.name.as_str())
        .collect()
}

/// Validate that a service name exists in the discovered services.
fn validate_service_name(
    name: &str,
    services: &[Service],
    mysql_services: &[Service],
) -> Result<()> {
    let all_names = all_service_names(services, mysql_services);
    if !all_names.contains(&name) {
        bail!("Unknown service: {name}");
    }
    Ok(())
}

fn handle_action(
    action: &str,
    service: Option<String>,
    all: bool,
    config_dir: &Path,
) -> Result<()> {
    let services = discover_services(config_dir);
    let mysql_services = discover_mysql_services(config_dir);

    let targets: Vec<String> = if all {
        // Dependencies first for start/restart, dependents first for stop
        let ordered: Box<dyn Iterator<Item = &Service>> = if action == "stop" {
            Box::new(services.iter().chain(mysql_services.iter()))
        } else {
            Box::new(mysql_services.iter().chain(services.iter()))
        };
        ordered.map(|s| s.name.clone()).collect()
    } else {
        match service {
            Some(name) => {
                validate_service_name(&name, &services, &mysql_services)?;
                vec![name]
            }
            None => bail!("Specify a service name or use --all"),
        }
    };

    for name in &targets {
        action_service(action, name, mysql_services.len())
            .with_context(|| format!("Failed to {action} {name}"))?;
        println!("{action} {name}: ok");
    }

    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let config_dir = cli.config_dir.unwrap_or_else(default_config_dir);

    match cli.command {
        Commands::Status { service, tree } => {
            let services = discover_services(&config_dir);
            let mysql_services = discover_mysql_services(&config_dir);

            if let Some(name) = service {
                validate_service_name(&name, &services, &mysql_services)?;
                match get_service_status(&name) {
                    Some(status) => println!("{:<22} {}", name, status),
                    None => println!("{:<22} not found", name),
                }
            } else if tree {
                let tree = build_tree(&services, &mysql_services);
                tree.print(&[]);
            } else {
                let mut grouped: BTreeMap<Tier, Vec<(&str, Option<ServiceStatus>)>> =
                    BTreeMap::new();

                for s in &services {
                    grouped
                        .entry(s.tier)
                        .or_default()
                        .push((s.name.as_str(), get_service_status(&s.name)));
                }
                for s in &mysql_services {
                    grouped
                        .entry(s.tier)
                        .or_default()
                        .push((s.name.as_str(), get_service_status(&s.name)));
                }

                for (tier, entries) in &grouped {
                    println!("{tier}");
                    for (name, status) in entries {
                        let status_str = match status {
                            Some(s) => s.to_string(),
                            None => "not found".to_string(),
                        };
                        println!("  {name:<22} {status_str}");
                    }
                    println!();
                }
            }
        }
        Commands::Start { service, all } => handle_action("start", service, all, &config_dir)?,
        Commands::Stop { service, all } => handle_action("stop", service, all, &config_dir)?,
        Commands::Restart { service, all } => handle_action("restart", service, all, &config_dir)?,
        Commands::List => {
            let services = discover_services(&config_dir);
            let mysql_services = discover_mysql_services(&config_dir);

            println!("Discovered services:");
            for s in &services {
                println!("  {} ({})", s.name, s.tier);
            }
            for s in &mysql_services {
                println!("  {} ({})", s.name, s.tier);
            }
        }
    }

    Ok(())
}

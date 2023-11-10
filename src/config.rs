
use std::{
    collections::HashMap,
    str::FromStr,
    path::Path,
};

use anyhow::Error;
use serde::{Serialize, Deserialize};


const DEFAULT_CONFIG_STR: &str = include_str!("methd.default.toml");

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct DaemonConfig {
    pub endpoint: Option<String>,
    pub key_path: Option<String>,
    pub config_dir: Option<String>,
}

impl DaemonConfig {
    pub fn merge(&self, other: &Self) -> Self {
        DaemonConfig {
            endpoint: if let Some(endpoint) = &other.endpoint {
                Some(endpoint.to_owned())
            } else {
                self.endpoint.clone()
            },
            key_path: if let Some(key_path) = &other.key_path {
                Some(key_path.to_owned())
            } else {
                self.key_path.clone()
            },
            config_dir: if let Some(config_dir) = &other.config_dir {
                Some(config_dir.to_owned())
            } else {
                self.config_dir.clone()
            },
        }
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct PeerConfig {
    pub public_key: String,
    pub endpoint: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Config {
    pub daemon: Option<DaemonConfig>,
    pub peers: Option<HashMap<String, PeerConfig>>,
}

impl Config {
    pub fn load_from_path<P: AsRef<Path> + ToString>(path: P) -> Self {
        let s = std::fs::read_to_string(&path);
        let s = if let Ok(s) = s {
            s
        } else {
            log::warn!("Could not load the configuration: {}", path.to_string());
            return Config::default();
        };
        let config = Config::from_str(&s);
        let config = if let Ok(config) = config {
            config
        } else {
            log::warn!("Configuration syntax problems: {}", path.to_string());
            return Config::default();
        };

        let default_config = Config::default();
        let mut config = default_config.merge(&config);
        let daemon_config = config.daemon.as_ref().unwrap();
        let config_dir = daemon_config.config_dir.as_ref().unwrap();
        let parent_config_dir = path.as_ref().parent().unwrap_or(&Path::new("/"));
        let config_dir = parent_config_dir.join(config_dir);
        let config_pattern = format!("{}/*.toml", config_dir.to_string_lossy());
        let config_paths = glob::glob(&config_pattern).map(|paths| paths.collect()).unwrap_or(Vec::new()).into_iter().filter_map(Result::ok);
        for config_path in config_paths {
            let s = std::fs::read_to_string(config_path);
            let s = if let Ok(s) = s {
                s
            } else {
                continue;
            };
            let child_config = Config::from_str(&s);
            if let Ok(child_config) = child_config {
                config = config.merge(&child_config);
            }
        }
        config
    }

    pub fn merge(&self, other: &Self) -> Self {
        let daemon = if let Some(self_daemon) = &self.daemon {
            if let Some(other_daemon) = &other.daemon {
                Some(self_daemon.merge(&other_daemon))
            } else {
                Some(self_daemon.clone())
            }
        } else {
            other.daemon.clone()
        };
        let mut peers: HashMap<String, PeerConfig> = HashMap::new();
        if let Some(self_peers) = &self.peers {
            for (name, peer) in self_peers {
                peers.insert(name.to_owned(), peer.clone());
            }
        }
        if let Some(other_peers) = &other.peers {
            for (name, peer) in other_peers {
                peers.insert(name.to_owned(), peer.clone());
            }
        }
        Config {
            daemon: daemon,
            peers: Some(peers),
        }
    }
}

impl FromStr for Config {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Error> {
        Ok(toml::from_str(s)?)
    }
}

impl ToString for Config {
    fn to_string(&self) -> String {
        toml::to_string(self).unwrap()
    }
}

impl Default for Config {
    fn default() -> Self {
        toml::from_str(DEFAULT_CONFIG_STR).unwrap()
    }
}


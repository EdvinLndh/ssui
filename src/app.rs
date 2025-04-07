use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    prelude::Backend,
    style::Stylize,
    text::Line,
    widgets::{ListItem, ListState},
    Terminal,
};
use std::fs::File;
use std::io::Read;
use thiserror::Error;

use crate::ui;

#[derive(Default)]
pub struct SshConf {
    pub confs: Vec<Host>,
    pub state: ListState,
}

impl std::fmt::Display for SshConf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for host in &self.confs {
            // always display host_id
            writeln!(f, "{}", host)?;
        }
        Ok(())
    }
}

impl std::fmt::Display for Host {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // always display host_id
        writeln!(f, "host {}", self.host_id)?;

        // helper macro to write fields if they exist
        macro_rules! write_if_some {
            ($field:expr, $label:literal) => {
                if let Some(value) = $field {
                    writeln!(f, "    {} {}", $label, value)?;
                } else {
                    writeln!(f, "    {} none", $label)?;
                }
            };
        }

        write_if_some!(&self.host_name, "hostname");
        write_if_some!(self.port, "port");
        write_if_some!(&self.user, "user");
        write_if_some!(&self.proxy_jump, "proxyjump");
        write_if_some!(&self.local_forward, "localforward");
        write_if_some!(&self.id_file, "identityfile");

        Ok(())
    }
}

impl FromIterator<Host> for SshConf {
    fn from_iter<T: IntoIterator<Item = Host>>(iter: T) -> Self {
        let mut conf = SshConf::new();
        for host in iter {
            conf.confs.push(host);
        }
        conf
    }
}

#[derive(Debug, Error)]
pub enum SshConfError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Parse error on line {0}: {1}")]
    ParseError(usize, String),
}

pub struct Host {
    host_id: String,
    host_name: Option<String>,
    port: Option<u32>,
    user: Option<String>,
    proxy_jump: Option<String>,
    local_forward: Option<String>,
    id_file: Option<String>,
    expanded: bool,
}

enum SshSetting {
    ProxyJump,
    HostName,
    User,
    Identityfile,
    LocalForward,
    Port,
}

impl SshSetting {
    fn from_str(str: &str) -> Option<SshSetting> {
        match str.to_lowercase().as_str() {
            "port" => Some(SshSetting::Port),
            "hostname" => Some(SshSetting::HostName),
            "proxyjump" => Some(SshSetting::ProxyJump),
            "localforward" => Some(SshSetting::LocalForward),
            "identityfile" => Some(SshSetting::Identityfile),
            "user" => Some(SshSetting::User),
            _ => None,
        }
    }
}

impl SshConf {
    pub fn new() -> SshConf {
        SshConf {
            confs: Vec::new(),
            state: ListState::default(),
        }
    }

    fn parse(content: &str) -> Result<SshConf, SshConfError> {
        let mut confs = Vec::new();

        let mut host: Option<Host> = None;
        for (line_num, line) in content.lines().enumerate() {
            let line = line.trim();
            let host_id;

            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let host_str = "Host ";
            // Handle host declarations
            if line.starts_with(host_str) {
                // Save previous host if exists
                if let Some(h) = host.take() {
                    confs.push(h);
                }

                let patterns: Vec<String> = line[host_str.len()..]
                    .split_whitespace()
                    .map(|s| s.to_string())
                    .collect();
                // TODO Fix unwrap
                host_id = patterns.first().unwrap().clone();

                host = Some(Host {
                    host_id,
                    host_name: None,
                    port: None,
                    user: None,
                    proxy_jump: None,
                    local_forward: None,
                    id_file: None,
                    expanded: false,
                })
            }
            // Handle settings
            else if let Some(host) = &mut host {
                let parts: Vec<&str> = line.splitn(2, char::is_whitespace).collect();
                if parts.len() != 2 {
                    return Err(SshConfError::ParseError(
                        line_num + 1,
                        "Invalid config line format".to_string(),
                    ));
                }

                if let Some(setting) = SshSetting::from_str(parts[0]) {
                    match setting {
                        SshSetting::ProxyJump => {
                            host.proxy_jump = Some(parts[1].to_string().clone())
                        }
                        SshSetting::HostName => host.host_name = Some(parts[1].to_string().clone()),
                        SshSetting::User => host.user = Some(parts[1].to_string().clone()),
                        SshSetting::Identityfile => {
                            host.id_file = Some(parts[1].to_string().clone())
                        }
                        SshSetting::LocalForward => {
                            host.local_forward = Some(parts[1].to_string().clone())
                        }
                        SshSetting::Port => host.port = parts[1].parse().ok(),
                    };
                } else {
                    return Err(SshConfError::ParseError(
                        line_num + 1,
                        "Invalid config line format".to_string(),
                    ));
                }
            } else {
                return Err(SshConfError::ParseError(
                    line_num + 1,
                    "Setting outside of Host block".to_string(),
                ));
            }
        }

        // Add the last host if exists
        if let Some(host) = host.take() {
            confs.push(host);
        }

        Ok(SshConf {
            confs,
            state: ListState::default(),
        })
    }
}

#[derive(Default)]
pub struct App {
    pub confs: SshConf,
    selected_id: Option<String>,
}

impl App {
    pub fn new() -> App {
        App {
            confs: SshConf::new(),
            selected_id: None,
        }
    }

    // Populate confs attribute
    pub fn read_ssh_conf(self: &mut App) -> Result<(), Box<dyn std::error::Error>> {
        // TODO Better error handling on opening file
        let mut file = File::open("/home/edvin/.ssh/config")?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        self.confs = SshConf::parse(&contents)?;

        Ok(())
    }

    pub fn run<B: Backend>(
        &mut self,
        terminal: &mut Terminal<B>,
    ) -> Result<String, Box<dyn std::error::Error>> {
        loop {
            terminal.draw(|frame| ui::render(frame, self))?;
            if self.handle_keys()? {
                if let Some(selected) = &self.selected_id {
                    return Ok(selected.to_string());
                } else {
                    return Err(color_eyre::eyre::eyre!("No ssh config selected!").into());
                }
            }
        }
    }

    fn handle_keys(&mut self) -> Result<bool, Box<dyn std::error::Error>> {
        if let Event::Key(k) = event::read()? {
            if k.kind != event::KeyEventKind::Press {
                return Ok(false);
            }
            match k.code {
                KeyCode::Char('q') | KeyCode::Esc => return Ok(true),
                KeyCode::Char('h') | KeyCode::Left => self.confs.state.select(None),
                KeyCode::Char('j') | KeyCode::Down => self.confs.state.select_next(),
                KeyCode::Char('k') | KeyCode::Up => self.confs.state.select_previous(),
                KeyCode::Char('g') | KeyCode::Home => self.confs.state.select_first(),
                KeyCode::Char('G') | KeyCode::End => self.confs.state.select_last(),
                KeyCode::Char('l') | KeyCode::Right => {
                    self.expand();
                }
                KeyCode::Enter => {
                    if let Some(i) = self.confs.state.selected() {
                        self.selected_id = Some(self.confs.confs[i].host_id.clone());
                    };
                    return Ok(true);
                }
                _ => return Ok(false),
            };
        };
        Ok(false)
    }

    fn expand(&mut self) {
        if let Some(i) = self.confs.state.selected() {
            self.confs.confs[i].expanded = !self.confs.confs[i].expanded;
        }
    }
}

impl From<&Host> for ListItem<'_> {
    fn from(value: &Host) -> Self {
        // Create styled main line with host_id

        if !value.expanded {
            // Add host details as spans
            let mut line = Line::from(value.host_id.clone().bold());
            let details = vec![
                value.host_name.as_ref().map(|n| format!("🖥️ {}", n)),
                value.user.as_ref().map(|u| format!("👤 {}", u)),
                value.port.map(|p| format!("🚪 {}", p)),
                value.proxy_jump.as_ref().map(|p| format!("↗️↗️{}", p)),
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join("  ");

            if !details.is_empty() {
                line.push_span("  ");
                line.push_span(details);
            }
            ListItem::new(line)
        } else {
            let mut text = vec![Line::from(value.host_id.clone().bold().underlined())];

            macro_rules! attr_some {
                ($attr_name:expr, $field:literal) => {
                    if let Some(val) = $attr_name {
                        text.push(Line::from(format!("  {} {}", $field, val).clone()));
                    }
                };
            }

            attr_some!(&value.host_name, "HostName");
            attr_some!(value.port, "Port");
            attr_some!(&value.proxy_jump, "ProxyJump");
            attr_some!(&value.user, "User");
            attr_some!(&value.local_forward, "LocalForward");
            attr_some!(&value.id_file, "IdentityFile");

            ListItem::new(text)
        }
    }
}

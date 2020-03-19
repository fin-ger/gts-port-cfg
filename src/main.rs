use clap::{App, Arg};
use itertools::Itertools;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs::File;
use std::net::{IpAddr, Ipv4Addr};
use std::usize::MAX;

#[derive(Debug, Clone, PartialEq, Deserialize)]
struct Host {
    hostname: String,
    ip: IpAddr,
    #[serde(skip, default = "default_ip")]
    internal_ip: IpAddr,
    #[serde(skip)]
    devices: HashMap<String, String>,
    location: String,
    #[serde(default = "default_flavor")]
    flavor: String,
    #[serde(default = "default_image")]
    image: String,
    #[serde(default = "default_ports")]
    free_ports: usize,
    #[serde(skip)]
    used_ports: usize,
}

#[derive(Debug, PartialEq, Deserialize)]
struct Config {
    testbed: Testbed,
}

#[derive(Debug, PartialEq, Deserialize)]
struct Testbed {
    id: String,
    description: String,
    hosts: Vec<Host>,
}

fn default_ports() -> usize {
    0
}

fn default_ip() -> IpAddr {
    IpAddr::V4(Ipv4Addr::LOCALHOST)
}

fn default_image() -> String {
    "Ubuntu-18.04.qcow2".to_string()
}

fn default_flavor() -> String {
    "c2r2h20".to_string()
}

#[derive(Clone, Debug)]
struct Adjacency {
    host1_id: String,
    host1_port: usize,
    link_name: String,
    link_side: String,
}

#[derive(PartialEq, Debug)]
struct Link {
    id: String,
}

#[derive(Debug)]
struct DSLConfig {
    description: String,
    id: String,
    hosts: Vec<Host>,
    link: Vec<Link>,
    adjacents: Vec<Adjacency>,
}

impl Into<DSLConfig> for Config {
    fn into(self) -> DSLConfig {
        let mut adjacences = Vec::new();
        let mut new_hosts = self.testbed.hosts.clone();
        let mut old_hosts = self.testbed.hosts.clone();
        let mut link_count: usize = 0;
        for (pos, host) in new_hosts.iter_mut().enumerate() {
            host.used_ports += old_hosts[pos].used_ports;
            for other in old_hosts.iter_mut().skip(pos + 1) {
                host.used_ports += 1;
                other.used_ports += 1;
                link_count += 1;
                adjacences.push(Adjacency {
                    host1_id: host.hostname.clone(),
                    host1_port: host.used_ports,
                    link_name: format!("link{}", link_count),
                    link_side: "src".to_string(),
                });
                adjacences.push(Adjacency {
                    host1_id: other.hostname.clone(),
                    host1_port: other.used_ports,
                    link_name: format!("link{}", link_count),
                    link_side: "dst".to_string(),
                });
            }
        }

        let link_list = adjacences
            .iter()
            .map(|adj| Link {
                id: adj.link_name.clone(),
            })
            .dedup()
            .collect();

        for host in new_hosts.iter_mut() {
            let host_name = host.hostname.clone();
            for (num, link) in adjacences
                .iter()
                .filter(|elem| elem.host1_id == host_name)
                .enumerate()
            {
                host.devices
                    .insert(format!("ens{}", num + 7), link.link_name.clone());
            }
        }

        DSLConfig {
            description: self.testbed.description,
            id: self.testbed.id,
            hosts: new_hosts,
            link: link_list,
            adjacents: adjacences,
        }
    }
}

impl Link {
    fn serialize(&self) -> String {
        let mut out = String::new();
        out.push_str("    link {\n");
        out.push_str(format!("        id = \"{}\"\n", &self.id).as_str());
        out.push_str("        port { id = \"src\"}\n");
        out.push_str("        port { id = \"dst\"}\n");
        out.push_str("    }\n");
        out
    }
}

impl DSLConfig {
    fn serialize(&self) -> String {
        let mut out = String::new();
        out.push_str(format!("{} {{\n", self.id).as_str());
        out.push_str(format!("    id = \"{}\"\n", self.id).as_str());
        out.push_str(format!("    description = \"{}\"\n", self.description).as_str());

        for host in &self.hosts {
            out.push_str(host.serialize().as_str());
        }
        for link in &self.link {
            out.push_str(link.serialize().as_str());
        }
        for adjacence in &self.adjacents {
            out.push_str(
                format!(
                    "    adjacency {}.p{}, {}.{}\n",
                    adjacence.host1_id,
                    //offset to index
                    adjacence.host1_port + 1,
                    adjacence.link_name,
                    adjacence.link_side
                )
                .as_str(),
            );
        }
        out.push_str("}\n");
        out
    }
}

impl Host {
    fn serialize(&self) -> String {
        let mut out = String::new();
        out.push_str("    host {\n");
        out.push_str(format!("        id = \"{}\"\n", &self.hostname).as_str());
        out.push_str(format!("        location = \"{}\"\n", &self.location).as_str());
        out.push_str(format!("        imageId = \"{}\"\n", &self.image).as_str());
        out.push_str(format!("        flavorId = \"{}\"\n", &self.flavor).as_str());
        // Add open ports
        for port in 1..=(self.free_ports + self.used_ports + 1) {
            out.push_str(format!("        port {{ id = \"p{}\" }}\n", port).as_str());
        }
        out.push_str("    }\n");
        out
    }
}

fn load_config(path: &str) -> Config {
    let config_file = match File::open(path) {
        Ok(file) => file,
        Err(err) => {
            eprintln!("ERROR: could not open file '{}': {}", path, err);
            std::process::exit(1);
        }
    };
    let config: Config = match serde_yaml::from_reader(config_file) {
        Ok(config) => config,
        Err(err) => {
            eprintln!("ERROR: could not parse config file '{}': {}", path, err);
            std::process::exit(2);
        }
    };
    config
}

fn script_generation_ports(mut config: DSLConfig) {
    for (i, host) in config.hosts.iter_mut().enumerate() {
        host.internal_ip = IpAddr::V4(Ipv4Addr::new(10, 42, 42, i as u8 + 1));
    }

    let mut script = String::from(
        "#! /bin/sh
IP_ADDRESS=$(ip -f inet addr show ens3 | sed -En -e 's/.*inet ([0-9.]+).*/\\1/p')

case $IP_ADDRESS in
",
    );

    for host in config.hosts.iter() {
        script.push_str(format!("  {host_ip})\n", host_ip = host.ip).as_str());
        script.push_str(
            format!(
                "    sudo hostnamectl set-hostname {hostname}\n",
                hostname = host.hostname,
            )
            .as_str(),
        );

        for (device, link) in host.devices.iter() {
            let link_dsts: Vec<&Host> = config
                .hosts
                .iter()
                .filter(|h| h.devices.values().any(|l| l == link) && h.hostname != host.hostname)
                .collect();

            match link_dsts.len() {
                0 => {
                    eprintln!(
                        "ERROR: link {link} is only connected to host {host}",
                        link = link,
                        host = host.hostname,
                    );
                    std::process::exit(3);
                }
                2..=MAX => {
                    eprintln!(
                        "ERROR: link {link} is connected to too many hosts {hosts}",
                        link = link,
                        hosts = link_dsts.iter().map(|h| &h.hostname).format(", "),
                    );
                    std::process::exit(3);
                }
                _ => {}
            }

            script.push_str(
                format!(
                    "
    sudo ip link set dev {host_device} down
    sudo ip addr flush dev {host_device}
    sudo ip addr add {host_internal_ip} dev {host_device}
    sudo ip link set dev {host_device} up
    sudo ip route add {host_link_dst} dev {host_device}
",
                    host_internal_ip = host.internal_ip,
                    host_device = device,
                    host_link_dst = link_dsts[0].internal_ip,
                )
                .as_str(),
            );
        }

        script.push_str("    ;;\n");
    }
    script.push_str("esac\n");

    println!("{}", script);
}

fn main() {
    let matches = App::new("gts-port-cfg")
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(
            Arg::with_name("CONFIG")
                .required(true)
                .index(1)
                .help("the YAML configuration file containing the GTS topology")
                .long_help(
                    "The YAML configuration file describes the topology running in GTS.
The structure of the file looks like this:

testbed:
  id: foobar
  description: Foo that bar
  hosts:
    - hostname: node1
      ip: 172.16.0.108
      location: ams
      free_ports: 1
    - hostname: node2
      ip: 172.16.0.109
      location: ams
      flavor: c4r4h20
    - hostname: node3
      ip: 172.16.0.106
      location: ham
      image: FreeBSD-11.qcow2
    - hostname: node4
      ip: 172.16.0.107
      location: ams

The `ip` property is identifying the machines. The hostname will be set for the
machine. The link names specify which host is connected over which interface.
The `image`, `free_ports`, and `flavor` properties are optional and have to be
set to valid values provided by gts. When they are not set, a default value will
be chosen by GTS.",
                ),
        )
        .arg(
            Arg::with_name("DSL")
                .help("generate the DSL to allocate the hosts")
                .short("d")
                .long("dsl"),
        )
        .get_matches();
    let config_filename = matches.value_of("CONFIG").unwrap();

    let config = load_config(config_filename);
    let dsl: DSLConfig = config.into();

    if matches.is_present("DSL") {
        println!("{}", dsl.serialize());
    } else {
        script_generation_ports(dsl);
    }
}

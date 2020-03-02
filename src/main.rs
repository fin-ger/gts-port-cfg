use clap::{App, Arg};
use itertools::Itertools;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::net::{IpAddr, Ipv4Addr};
use std::usize::MAX;

#[derive(Debug, PartialEq, Deserialize)]
struct Host {
    hostname: String,
    ip: IpAddr,
    #[serde(skip, default = "default_ip")]
    internal_ip: IpAddr,
    devices: HashMap<String, String>,
    location: String,
    #[serde(default = "default_flavor")]
    flavor: String,
    #[serde(default = "default_image")]
    image: String,
    #[serde(default = "default_ports")]
    ports: usize,
}

#[derive(Debug, PartialEq, Deserialize)]
struct Config {
    testbed: Testbed,
    hosts: Vec<Host>,
}

#[derive(Debug, PartialEq, Deserialize)]
struct Testbed {
    id: String,
    description: String,
}

fn default_ports() -> usize {
    28
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

#[derive(Clone)]
struct Adjacency {
    host1_id: String,
    host1_port: usize,
    link_name: String,
    link_side: String,
}

struct Link {
    id: String,
}

struct DSLConfig {
    description: String,
    id: String,
    hosts: Vec<Host>,
    link: Vec<Link>,
    adjacents: Vec<Adjacency>,
}

impl Into<DSLConfig> for Config {
    fn into(self) -> DSLConfig {
        let link_names = self.hosts.iter().fold(HashSet::new(), |mut acc, x| {
            for (_, link) in x.devices.iter() {
                acc.insert(link);
            }
            acc
        });

        let link_list = link_names
            .iter()
            .map(|x| Link {
                id: (*x).to_string(),
            })
            .collect();

        let link_props = self.hosts.iter().fold(Vec::new(), |mut acc, x| {
            let mut ord_links = x
                .devices
                .iter()
                .fold(Vec::new(), |mut links, (_, link_name)| {
                    links.push(link_name);
                    links
                });
            ord_links.sort();
            acc.push((&x.hostname, ord_links));
            acc
        });

        let adjacencies: Vec<Vec<Adjacency>> = link_names.iter().map(|elem| {
            let list_of_links: Vec<&(&String, Vec<&String>)> = link_props.iter().filter(|(_, links)| links.contains(elem)).collect();
            match  list_of_links.len() {
                0 => {
                    eprintln!("Link of name {} is not part of any host.", elem);
                    std::process::exit(3)
                },
                1 => {
                    eprintln!("Link of name {} is only connected to one host. Name the Link in another hosts device to connect them.", elem);
                    std::process::exit(4)
                },
                2 => {
                    vec![
                    Adjacency {
                        host1_id: list_of_links[0].0.clone(),
                        host1_port: list_of_links[0].1.binary_search(elem).unwrap(),
                        link_name: (*elem).to_string(),
                        link_side: "src".to_string(),
                    },
                    Adjacency {
                        host1_id: list_of_links[1].0.clone(),
                        host1_port: list_of_links[1].1.binary_search(elem).unwrap(),
                        link_name: (*elem).to_string(),
                        link_side: "dst".to_string(),
                    },
                    ]
                }
                _ => {
                    eprintln!("To many hosts are connected to link {}", elem);
                    std::process::exit(5)
                },
            }
        }).collect();

        DSLConfig {
            description: self.testbed.description,
            id: self.testbed.id,
            hosts: self.hosts,
            link: link_list,
            adjacents: adjacencies.concat(),
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
        for port in 1..=self.ports {
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

fn script_generation_ports(mut config: Config) {
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

hosts:
  - hostname: host1
    ip: 172.16.0.108
    devices:
      ens6: link1
      ens7: link2
      ens8: link3
  - hostname: host2
    ip: 172.16.0.109
    devices:
      ens6: link1
      ens7: link5
      ens8: link6
  - hostname: host3
    ip: 172.16.0.106
    devices:
      ens6: link3
      ens7: link4
      ens8: link5
  - hostname: host4
    ip: 172.16.0.107
    devices:
      ens6: link2
      ens7: link4
      ens8: link6

The `ip` property is identifying the machines. The hostname will be set for the machine.
The link names specify which host is connect over which interface.",
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

    if matches.is_present("DSL") {
        let dsl: DSLConfig = config.into();
        println!("{}", dsl.serialize());
    } else {
        script_generation_ports(config);
    }
}

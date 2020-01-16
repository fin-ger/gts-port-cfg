use std::net::{IpAddr, Ipv4Addr};
use std::fs::File;
use std::collections::HashMap;
use std::usize::MAX;
use clap::{Arg, App};
use serde::Deserialize;
use itertools::Itertools;

fn default_ip() -> IpAddr {
    IpAddr::V4(Ipv4Addr::LOCALHOST)
}

#[derive(Debug, PartialEq, Deserialize)]
struct Host {
    hostname: String,
    ip: IpAddr,
    #[serde(skip, default = "default_ip")]
    internal_ip: IpAddr,
    devices: HashMap<String, String>,
}

#[derive(Debug, PartialEq, Deserialize)]
struct Config {
    hosts: Vec<Host>,
}

fn main() {
    let matches = App::new("gts-port-cfg")
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(Arg::with_name("CONFIG")
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
The link names specify which host is connect over which interface."))
        .get_matches();
    let config_filename = matches.value_of("CONFIG").unwrap();
    let config_file = match File::open(config_filename) {
        Ok(file) => file,
        Err(err) => {
            eprintln!("ERROR: could not open file '{}': {}", config_filename, err);
            std::process::exit(1);
        }
    };
    let mut config: Config = match serde_yaml::from_reader(config_file) {
        Ok(config) => config,
        Err(err) => {
            eprintln!("ERROR: could not parse config file '{}': {}", config_filename, err);
            std::process::exit(2);
        }
    };


    for (i, host) in config.hosts.iter_mut().enumerate() {
        host.internal_ip = IpAddr::V4(Ipv4Addr::new(10, 42, 42, i as u8 + 1));
    }

    let mut script = String::from("#! /bin/sh
IP_ADDRESS=$(ip -f inet addr show ens3 | sed -En -e 's/.*inet ([0-9.]+).*/\\1/p')

case $IP_ADDRESS in
");

    for host in config.hosts.iter() {
        script.push_str(format!("  {host_ip})\n", host_ip = host.ip).as_str());
        script.push_str(format!(
            "    sudo hostnamectl set-hostname {hostname}\n",
            hostname = host.hostname,
        ).as_str());

        for (device, link) in host.devices.iter() {
            let link_dsts: Vec<&Host> = config.hosts.iter()
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
                },
                2..=MAX => {
                    eprintln!(
                        "ERROR: link {link} is connected to too many hosts {hosts}",
                        link = link,
                        hosts = link_dsts.iter().map(|h| &h.hostname).format(", "),
                    );
                    std::process::exit(3);
                },
                _ => {},
            }

            script.push_str(format!(
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
            ).as_str());
        }

        script.push_str("    ;;\n");
    }
    script.push_str("esac\n");


    println!("{}", script);
}

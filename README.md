<h1 align="center">Welcome to gts-port-cfg 👋</h1>
<p align="center">
  <a href="https://github.com/fin-ger/gts-port-cfg/actions?query=workflow%3A%22rust+stable+build%22">
    <img src="https://github.com/fin-ger/gts-port-cfg/workflows/rust stable build/badge.svg" alt="rust stable build">
  </a>
  <a href="https://github.com/fin-ger/gts-port-cfg/actions?query=workflow%3A%22rust+nightly+build%22">
    <img src="https://github.com/fin-ger/gts-port-cfg/workflows/rust nightly build/badge.svg" alt="rust nightly build">
  </a>
  <a href="https://github.com/fin-ger/gts-port-cfg/blob/master/LICENSE">
    <img alt="GitHub" src="https://img.shields.io/github/license/fin-ger/gts-port-cfg">
  </a>
  <a href="http://spacemacs.org">
    <img src="https://cdn.rawgit.com/syl20bnr/spacemacs/442d025779da2f62fc86c2082703697714db6514/assets/spacemacs-badge.svg" />
  </a>
  <a href="http://makeapullrequest.com">
    <img alt="PRs Welcome" src="https://img.shields.io/badge/PRs-welcome-brightgreen.svg">
  </a>
  <br>
  <i>A local network setup for
  <a href="https://www.geant.org/Services/Connectivity_and_network/GTS">GTS Géant</a>
  testbeds</i>
</p>

---

This program generates a local network setup script which creates a local network for full-mesh connected GTS Géant testbeds from a YAML configuration file.

<a href="https://github.com/fin-ger/gts-port-cfg/releases/latest/download/gts-port-cfg">
  <img src="https://img.shields.io/badge/download-linux-blue?style=for-the-badge&logo=linux" alt="Download">
</a>

## Usage

The configuration for 4 fully connected nodes looks like this:

```yaml
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
```

> The `image`, `free_ports` and `flavor` properties are optional and have to be set to the given values from gts, when they are not set a default value will be chosen instead.

### Known issues

 - IP addresses are not respected by the GTS DSL

### Script

Use this configuration file to generate a shell script which sets up a `10.42.42.0/24` network for these nodes.

```bash
$ gts-port-cfg config.yml
#! /bin/sh
IP_ADDRESS=$(ip -f inet addr show ens3 | sed -En -e 's/.*inet ([0-9.]+).*/\1/p')

case $IP_ADDRESS in
  172.16.0.108)
    sudo hostnamectl set-hostname node1

    sudo ip link set dev ens6 down
    sudo ip addr flush dev ens6
    sudo ip addr add 10.42.42.1 dev ens6
    sudo ip link set dev ens6 up
    sudo ip route add 10.42.42.2 dev ens6

    sudo ip link set dev ens7 down
    sudo ip addr flush dev ens7
    sudo ip addr add 10.42.42.1 dev ens7
    sudo ip link set dev ens7 up
    sudo ip route add 10.42.42.3 dev ens7

    sudo ip link set dev ens8 down
    sudo ip addr flush dev ens8
    sudo ip addr add 10.42.42.1 dev ens8
    sudo ip link set dev ens8 up
    sudo ip route add 10.42.42.4 dev ens8
    ;;
  172.16.0.109)
    sudo hostnamectl set-hostname node2

    sudo ip link set dev ens8 down
    sudo ip addr flush dev ens8
    sudo ip addr add 10.42.42.2 dev ens8
    sudo ip link set dev ens8 up
    sudo ip route add 10.42.42.4 dev ens8

    sudo ip link set dev ens6 down
    sudo ip addr flush dev ens6
    sudo ip addr add 10.42.42.2 dev ens6
    sudo ip link set dev ens6 up
    sudo ip route add 10.42.42.1 dev ens6

    sudo ip link set dev ens7 down
    sudo ip addr flush dev ens7
    sudo ip addr add 10.42.42.2 dev ens7
    sudo ip link set dev ens7 up
    sudo ip route add 10.42.42.3 dev ens7
    ;;
  172.16.0.106)
    sudo hostnamectl set-hostname node3

    sudo ip link set dev ens8 down
    sudo ip addr flush dev ens8
    sudo ip addr add 10.42.42.3 dev ens8
    sudo ip link set dev ens8 up
    sudo ip route add 10.42.42.4 dev ens8

    sudo ip link set dev ens7 down
    sudo ip addr flush dev ens7
    sudo ip addr add 10.42.42.3 dev ens7
    sudo ip link set dev ens7 up
    sudo ip route add 10.42.42.2 dev ens7

    sudo ip link set dev ens6 down
    sudo ip addr flush dev ens6
    sudo ip addr add 10.42.42.3 dev ens6
    sudo ip link set dev ens6 up
    sudo ip route add 10.42.42.1 dev ens6
    ;;
  172.16.0.107)
    sudo hostnamectl set-hostname node4

    sudo ip link set dev ens6 down
    sudo ip addr flush dev ens6
    sudo ip addr add 10.42.42.4 dev ens6
    sudo ip link set dev ens6 up
    sudo ip route add 10.42.42.1 dev ens6

    sudo ip link set dev ens7 down
    sudo ip addr flush dev ens7
    sudo ip addr add 10.42.42.4 dev ens7
    sudo ip link set dev ens7 up
    sudo ip route add 10.42.42.2 dev ens7

    sudo ip link set dev ens8 down
    sudo ip addr flush dev ens8
    sudo ip addr add 10.42.42.4 dev ens8
    sudo ip link set dev ens8 up
    sudo ip route add 10.42.42.3 dev ens8
    ;;
esac
```

You can pipe the output of this program directly to `sh` to execute the `ip` commands.

### DSL

Use the `-d` flag to generate the according gts-dsl to your configuration. 
```
$ gts-port-cfg -d config.yml
foobar {
    id = "foobar"
    description = "Foo that bar"
    host {
        id = "node1"
        location = "ams"
        imageId = "Ubuntu-18.04.qcow2"
        flavorId = "c2r2h20"
        port { id = "p1" }
        port { id = "p2" }
        port { id = "p3" }
        port { id = "p4" }
    }
    host {
        id = "node2"
        location = "ams"
        imageId = "Ubuntu-18.04.qcow2"
        flavorId = "c4r4h20"
        port { id = "p1" }
        port { id = "p2" }
        port { id = "p3" }
    }
    host {
        id = "node3"
        location = "ham"
        imageId = "FreeBSD-11.qcow2"
        flavorId = "c2r2h20"
        port { id = "p1" }
        port { id = "p2" }
        port { id = "p3" }
    }
    host {
        id = "node4"
        location = "ams"
        imageId = "Ubuntu-18.04.qcow2"
        flavorId = "c2r2h20"
        port { id = "p1" }
        port { id = "p2" }
        port { id = "p3" }
    }
    link {
        id = "link1"
        port { id = "src"}
        port { id = "dst"}
    }
    link {
        id = "link2"
        port { id = "src"}
        port { id = "dst"}
    }
    link {
        id = "link3"
        port { id = "src"}
        port { id = "dst"}
    }
    link {
        id = "link4"
        port { id = "src"}
        port { id = "dst"}
    }
    link {
        id = "link5"
        port { id = "src"}
        port { id = "dst"}
    }
    link {
        id = "link6"
        port { id = "src"}
        port { id = "dst"}
    }
    adjacency node1.p2, link1.src
    adjacency node2.p2, link1.dst
    adjacency node1.p3, link2.src
    adjacency node3.p2, link2.dst
    adjacency node1.p4, link3.src
    adjacency node4.p2, link3.dst
    adjacency node2.p3, link4.src
    adjacency node3.p3, link4.dst
    adjacency node2.p4, link5.src
    adjacency node4.p3, link5.dst
    adjacency node3.p4, link6.src
    adjacency node4.p4, link6.dst
}
```
 
## Building the Project

Instead of downloading a precompiled binary, you can build the project yourself from source. First you have to setup a Rust toolchain. I recommend using [`rustup`](https://rustup.rs/). When the latest Rust stable toolchain is successfully installed, you can compile the code.

```
$ cargo install --path .
```

The program will be installed to `~/.cargo/bin/gts-port-cfg`.
 
## Troubleshooting

If you find any bugs/unexpected behaviour or you have a proposition for future changes open an issue describing the current behaviour and what you expected.

## Authors

**Fin Christensen**

> [:octocat: `@fin-ger`](https://github.com/fin-ger)  
> [:elephant: `@fin_ger@weirder.earth`](https://weirder.earth/@fin_ger)  
> [:bird: `@fin_ger_github`](https://twitter.com/fin_ger_github)  

<br>

**Johannes Wünsche**

> [:octocat: `@jwuensche`](https://github.com/jwuensche)  
> [:elephant: `@fredowald@mastodon.social`](https://mastodon.social/web/accounts/843376)  
> [:bird: `@Fredowald`](https://twitter.com/fredowald)  

## Show your support

Give a :star: if this project helped you!

# Cryptix-miner CPU only

A Rust binary for file encryption to multiple participants. Supports only local Mining on a local Node, no Stratum Support. Supports only the CPU, no GPU.

For a Http + Stratum Pool Support and GPU Support Miner use: 

[Cryptix Miner GPU & CPU](https://github.com/cryptix-network/cryptix-miner)


### Supports:
- Local Mining on Node via 127.0.0.1
- HTTP Mining on Node via Webaddress not supported now
- Stratum Mining on Pool not supported now

## Installation
### From Sources
With Rust's package manager cargo, you can install cryptix-miner via:

```sh
cargo install cryptix-miner
```

### From Binaries
The [release page](https://github.com/cryptix-network/cryptix-miner/releases) includes precompiled binaries for Linux, macOS and Windows.


# Usage
To start mining you need to run [cryptixd](https://github.com/cryptix-network/rusty-cryptix) and have an address to send the rewards to.

Help:
```
cryptix-miner 0.2.6
A Cryptix high performance CPU miner

USAGE:
    cryptix-miner [FLAGS] [OPTIONS] --mining-address <mining-address>

FLAGS:
    -d, --debug                   Enable debug logging level
    -h, --help                    Prints help information
        --mine-when-not-synced    Mine even when cryptixd says it is not synced, only useful when passing `--allow-submit-
                                  block-when-not-synced` to cryptixd  [default: false]
        --testnet                 Use testnet instead of mainnet [default: false]
    -V, --version                 Prints version information

OPTIONS:
    -s, --cryptixd-address <cryptixd-address>      The IP of the cryptixd instance [default: 127.0.0.1]
    -a, --mining-address <mining-address>      The Cryptix address for the miner reward
    -t, --threads <num-threads>                Amount of miner threads to launch [default: number of logical cpus]
    -p, --port <port>                          Cryptixd port [default: Mainnet = 19201, Testnet = 19202]
```

To start mining you just need to run the following:

`./cryptix-miner-cpu -s 127.0.0.1 -p 19201 --mining-address cryptix:XXXXX`


This will run the miner on all the available CPU cores.

`./cryptix-miner-cpu -s 127.0.0.1 -p 19201 --mining-address cryptix:XXXXX --threads 4 `

This will run the miner on 4 CPU cores.

## Discord

Join our discord server using the following link: [https://discord.cryptix-network.org/](https://discord.cryptix-network.org/)

# Devfund
The devfund is a fund managed by the Cryptix community in order to fund Cryptix development <br>
Devfund is 1%

# Donation Address & Kudos
Cryptis: `cryptix:qrjefk2r8wp607rmyvxmgjansqcwugjazpu2kk2r7057gltxetdvk8gl9fs0w`

 [elichai](https://github.com/elichai )`kaspa:qzvqtx5gkvl3tc54up6r8pk5mhuft9rtr0lvn624w9mtv4eqm9rvc9zfdmmpu`

# Kudos
 [elichai](https://github.com/elichai )

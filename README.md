# Cryptix-miner CPU only

A Rust binary for file encryption to multiple participants. This Miner supports only local Mining, no Stratum.


For a GPU & CPU Miner with Stratum Support use:


[Cryptix CPU & GPU Miner](https://github.com/cryptix-network/cryptix-miner/) 


## Installation
### From Sources
With Rust's package manager cargo, you can install cryptix-miner via:

```sh
cargo install cryptix-miner
```

### From Binaries
The [release page](https://github.com/cryptix-network/cryptix-miner-cpu/releases) includes precompiled binaries for Linux, macOS and Windows.


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
        --devfund <devfund-address>            Mine a percentage of the blocks to the Cryptix devfund [default: Off]
        --devfund-percent <devfund-percent>    The percentage of blocks to send to the devfund [default: 1]
    -s, --cryptixd-address <cryptixd-address>      The IP of the cryptixd instance [default: 127.0.0.1]
    -a, --mining-address <mining-address>      The Cryptix address for the miner reward
    -t, --threads <num-threads>                Amount of miner threads to launch [default: number of logical cpus]
    -p, --port <port>                          Cryptixd port [default: Mainnet = 19201, Testnet = 19202]
```

To start mining you just need to run the following:

`./cryptix-miner -s 127.0.0.1 -p 19201 --mining-address cryptix:XXXXX`


This will run the miner on all the available CPU cores.

`./cryptix-miner -s 127.0.0.1 -p 19201 --mining-address cryptix:XXXXX --threads 4 `

This will run the miner on 4 CPU cores.

# Devfund
The devfund is a fund managed by the Cryptix community in order to fund Cryptix development <br>
Devfund is 1%

# Donation Address & Kudos
`cryptix:cryptix:qrjefk2r8wp607rmyvxmgjansqcwugjazpu2kk2r7057gltxetdvk8gl9fs0w`

 [elichai](https://github.com/elichai) `kaspa:qzvqtx5gkvl3tc54up6r8pk5mhuft9rtr0lvn624w9mtv4eqm9rvc9zfdmmpu`

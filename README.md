Ellipticoind
==========

Ellipticoind is an Ellipticoin node written in Rust.


Building from source and running a miner:
==========================
1. Install rust

```
$ curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
$ source $HOME/.cargo/env
```

2. Clone the repo

```
$ git clone https://github.com/ellipticoin/ellipticoind.git
$ cd ellipticoind
```


3. Install the required dependencies

```
$ sudo apt-get update && sudo apt-get install build-essential libpq-dev pkg-config libssl-dev postgresql postgresql-contrib redis-server llvm clang redis git-lfs
```
4. Build  ellipticoind
```
$ cd ellipticoind
$ C_INCLUDE_PATH=/usr/lib/gcc/x86_64-linux-gnu/7/include cargo build --release
```
5. Generate a key pair:
```
$ ./target/release/ellipticoind generate-keypair
Public Key (Address): cwZitLN90FXTaOovm0ygsGNJ+nDJgFXg0Angzz7Lsbw=
Private Key: gNCgX1Jfs3gXHDEvd7ano6bflJR0oNscgBI1O4JEN2N06SFQL1isJysk3/ix35gkwG7MztBrGv2iO/q2Th7SnQ==
```

6. Copy the sample `.env` file and add your private key
```
$ cp .env.sample .env
```
# .env
DATABASE_URL=postgres://root:@/ellipticoind
PRIVATE_KEY=<Your Base 64 Encoded Private Key>
ENABLE_MINER=true
BURN_PER_BLOCK=100
```

7. Create the postgres user and database

```
#
$ cd /
$ su postgres -c"createuser root"
$ su postgres -c"createdb ellipticoind"
$ cd /root/ellipticoind 
```

8. Pull down the Ethereum Balances file from GitHub
```
$ git lfs install
$ git lfs pull
```
9. Run  ellipticoind (replace <your-external-ip> with your IP)

```
$ HOST="<your-external-ip>" ./target/release/ellipticoind
```

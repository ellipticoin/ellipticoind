Ellipticoind
==========

Ellipticoind is an Ellipticoin node written in Rust.


Building from source and running a miner:
==========================
1. Clone the repo

```
git clone https://github.com/ellipticoin/ellipticoind.git
```

2. Install rust

```
$ curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
$ source $HOME/.cargo/env
```

3. Install the required dependencies

```
$ sudo apt-get update && apt-get install install install build-essential libpq-dev pkg-config libssl-dev postgresql postgresql-contrib redis-server llvm clang redis git-lfs
```
6. Build  ellipticoind
```
$ cd ellipticoind
$ C_INCLUDE_PATH=/usr/lib/gcc/x86_64-linux-gnu/7/include cargo build
```



5. Generate a key pair:

```
$ cargo run -- generate-keypair
Public Key (Address): cwZitLN90FXTaOovm0ygsGNJ+nDJgFXg0Angzz7Lsbw=
Private Key: gNCgX1Jfs3gXHDEvd7ano6bflJR0oNscgBI1O4JEN2N06SFQL1isJysk3/ix35gkwG7MztBrGv2iO/q2Th7SnQ==
```

4. Paste the following into `.env`

```
# .env
DATABASE_URL=postgres://root:@/ellipticoind
PRIVATE_KEY=<Your Base 64 Encoded Private Key>
ENABLE_MINER=true
BURN_PER_BLOCK=100
```

5. Allow postgres connections from localhost

Make the following change to `/etc/postgresql/10/main/pg_hba.conf`:
```diff
# TYPE  DATABASE        USER            ADDRESS                 METHOD
- local   all           postgres                                md5
+ local   all           postgres                                peer
```
4. Create the postgres user and database

```
#
$ cd /
$ su postgres -c"createuser root"
$ su postgres -c"createdb ellipticoind"
$ cd /root/ellipticoind 
```

7. Pull down the Ethereum Balances file from GitHub
```
$ git lfs install
$ git lfs pull
```
6. Run  ellipticoind (replace <your-external-ip> with your IP)

```
$ cargo run -- --external-ip <your-external-ip>
```

Ellipticoind
==========

Ellipticoind is an Ellipticoin node written in Rust.


Building from source and running a miner:
==========================
1. Clone the repo

```
git clone git@github.com:ellipticoin/ellipticoind.git 
```

2. Install rust

```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

3. Install the required dependencies

```
$ sudo apt install install build-essential libpq-dev pkg-config libssl-dev postgresql postgresql-contrib redis llvm clang-dev redis
```

4. Create the postgres user and database

```
$ su postgres -c"createuser root"
$ createdb ellipticoind
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

6. Run  ellipticoind

```
$ cargo run --external-ip <your-ip>
```

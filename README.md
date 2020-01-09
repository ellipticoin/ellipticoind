Ellipticoind
==========

Ellipticoind is an Ellipticoin node written in Rust.

Installation (on Ubuntu 18 or similar):
==========================
1. Download ellipticoind

    $ wget http://davenport.ellipticoin.org/ellipticoind

2. Create an `ellipticoin` directory

  $ mkdir ellpiticoin && cd ellpiticoin


3. Install the required dependencies

  $ sudo apt install postgresql libpq-dev redis-server

4. Create the postgres user and database

  $ su postgres -c"createuser root"`
  $ createdb ellipticoind"

5. Generate a key pair:

  cargo run -- generate-keypair
  Public Key (Address): cwZitLN90FXTaOovm0ygsGNJ+nDJgFXg0Angzz7Lsbw=
Private Key: gNCgX1Jfs3gXHDEvd7ano6bflJR0oNscgBI1O4JEN2N06SFQL1isJysk3/ix35gkwG7MztBrGv2iO/q2Th7SnQ==


6. Paste the following into `.env`:

  # .env
  DATABASE_URL=postgres://root:@/ellipticoind
  PRIVATE_KEY=<Your Base64 Encoded Private Key from above>

7. Change the permissions on the binary to make it executable

  chmod 755 ellipticoind

8. Run `ellipticoind`!

  ./ellipticoind

Building and running from source:
==========================
1. Clone the repo

  git clone git@github.com:ellipticoin/ellipticoind.git 

2. Install rust

  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

3. Install the required dependencies

  $ sudo apt install postgresql libpq-dev redis-server

4. Create the postgres user and database

  $ su postgres -c"createuser root"`
  $ createdb ellipticoind"

5. Generate a key pair:

  cargo run -- generate-keypair
  Public Key (Address): cwZitLN90FXTaOovm0ygsGNJ+nDJgFXg0Angzz7Lsbw=
  Private Key: gNCgX1Jfs3gXHDEvd7ano6bflJR0oNscgBI1O4JEN2N06SFQL1isJysk3/ix35gkwG7MztBrGv2iO/q2Th7SnQ==

4. Paste the following into `.env`

  # .env
  DATABASE_URL=postgres://root:@/ellipticoind
  PRIVATE_KEY=<Your Base 64 Encoded Private Key>

5. Run  ellipticoind

  cargo run

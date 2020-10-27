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
$ sudo apt-get update && sudo apt-get install certbot nginx build-essential libpq-dev pkg-config libssl-dev postgresql postgresql-contrib redis-server llvm clang redis git-lfs
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

```
# .env
DATABASE_URL=postgres://root:@/ellipticoind
PRIVATE_KEY=<Your Base 64 Encoded Private Key>
ENABLE_MINER=true
BURN_PER_BLOCK=100
```

7. Create the postgres user and database

```
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
9. Run [certbot](https://certbot.eff.org/)
```
$ certbot --nginx -d yourhost.yourdomain.com
```
10. Comment out the following lines of  `/etc/nginx/sites-enabled/default`
```
#server {
#    if ($host = fritz.ellipticoin.org) {
#        return 301 https://$host$request_uri;
#    } # managed by Certbot


#       listen 80 ;
#       listen [::]:80 ;
#    server_name fritz.ellipticoin.org;
#    return 404; # managed by Certbot


#}
```
11. Find the `location` directive under the "SSL configuration" section of `/etc/nginx/sites-enabled/default` and add the following: 

        location / {
                proxy_pass http://upstream;
                proxy_buffering off;
                proxy_cache off;
                proxy_set_header Host $host;
                proxy_set_header Connection '';
                proxy_http_version 1.1;
                chunked_transfer_encoding off;
                # First attempt to serve request as file, then
                # as directory, then fall back to displaying a 404.
                try_files $uri $uri/ =404;
        }

12. Run  ellipticoind (replace yourhost.yourdomain.com with your domain)

```
$ HOST="yourhost.yourdomain.com" ./target/release/ellipticoind
```

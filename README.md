Ellipticoind
==========

Ellipticoind is an Ellipticoin node written in Rust.


Building from source and running a miner:
==========================
1. Install rust

```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

2. Clone the repo

```
git clone https://github.com/ellipticoin/ellipticoind.git
cd ellipticoind
```


3. Install the required dependencies

```
sudo apt-get update && sudo apt-get install certbot nginx build-essential pkg-config libssl-dev
```
4. Build  ellipticoind
```
cargo build --release
```
5. Generate a key pair:
```
cargo run generate-keypair
Public Key (Address): cwZitLN90FXTaOovm0ygsGNJ+nDJgFXg0Angzz7Lsbw=
Private Key: gNCgX1Jfs3gXHDEvd7ano6bflJR0oNscgBI1O4JEN2N06SFQL1isJysk3/ix35gkwG7MztBrGv2iO/q2Th7SnQ==
```

6. Copy the sample `.env` file and add your private key

```
cp .env.sample .env
```

```
# .env
DATABASE_URL=postgres://root:@/ellipticoind
PRIVATE_KEY=<Your Private Key>
ENABLE_MINER=true
BURN_PER_BLOCK=100
```

10. Run certbot
```
certbot --nginx -d yourhost.yourdomain.com
```
11. Add the following to `/etc/nginx/sites-enabled/default`
```
upstream upstream {
  server 127.0.0.1:80;
}

server {

        root /var/www/html;

        index index.html index.htm index.nginx-debian.html;
        server_name chicago.bitcoin.dance; # managed by Certbot

        location / {
                proxy_pass http://upstream;
                proxy_buffering off;
                proxy_cache off;
                proxy_set_header Host $host;
                proxy_set_header Connection '';
                proxy_http_version 1.1;
                chunked_transfer_encoding off;
                try_files $uri $uri/ =404;
        }

    listen [::]:443 ssl ipv6only=on; # managed by Certbot
    listen 443 ssl; # managed by Certbot
    ssl_certificate /etc/letsencrypt/live/chicago.bitcoin.dance/fullchain.pem; # managed by Certbot
    ssl_certificate_key /etc/letsencrypt/live/chicago.bitcoin.dance/privkey.pem; # managed by Certbot
    include /etc/letsencrypt/options-ssl-nginx.conf; # managed by Certbot
    ssl_dhparam /etc/letsencrypt/ssl-dhparams.pem; # managed by Certbot

}
```

12. Run  ellipticoind (replace yourhost.yourdomain.com with your domain)

```
$ HOST="yourhost.yourdomain.com" ./target/release/ellipticoind
```

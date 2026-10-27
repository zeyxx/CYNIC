# Internet Exposure of Forgejo (CYNIC)

## Purpose

This document describes the process to expose the sovereign Forgejo instance
(`forgejo.synapse.cynic`) to the public internet safely, using a reverse proxy,
DNS, and firewall rules. It is an operational runbook; no change is applied
without operator approval (`--yes` / `--apply`).

## Components

- **Forgejo node**: `forgejo` (LXC, service `forgejo-backend` on port 3000,
  admin TLS on `forgejo-api-tls`).
- **Reverse proxy**: nginx or Caddy on an edge host with a public IP.
- **DNS**: A/AAAA record for `forgejo.<domain>` (or `git.<domain>`) pointing to
  the edge host.
- **Firewall**: allow only HTTPS (443) and SSH (22) from the internet; keep
  Forgejo backend on the private mesh.

## 1. DNS

1. Choose a public FQDN, e.g. `forgejo.example.com`.
2. Create an A record (and AAAA if IPv6) pointing to the edge host public IP.
3. Verify resolution:
   ```sh
   dig +short forgejo.example.com
   ```
4. (Optional) Create a CNAME `git.example.com -> forgejo.example.com`.

## 2. Firewall (edge host)

Allow inbound:
- TCP 443 (HTTPS) from anywhere.
- TCP 22 (SSH) from trusted admin IPs only (or a bastion).

Deny inbound to Forgejo backend port (3000) from the internet; it must only be
reachable via the reverse proxy on localhost or the private mesh.

Example (ufw):
```sh
ufw default deny incoming
ufw allow 443/tcp
ufw allow from <admin_ip> to any port 22 proto tcp
ufw enable
```

## 3. Reverse proxy (nginx)

Install nginx and obtain a TLS certificate (Let's Encrypt):
```sh
apt-get install -y nginx certbot python3-certbot-nginx
certbot --nginx -d forgejo.example.com
```

Site configuration (`/etc/nginx/sites-available/forgejo`):
```nginx
server {
    listen 443 ssl http2;
    server_name forgejo.example.com;

    ssl_certificate     /etc/letsencrypt/live/forgejo.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/forgejo.example.com/privkey.pem;

    client_max_body_size 512M;

    location / {
        proxy_pass http://127.0.0.1:3000;
        proxy_set_header Host              $host;
        proxy_set_header X-Real-IP         $remote_addr;
        proxy_set_header X-Forwarded-For   $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_http_version 1.1;
        proxy_request_buffering off;
    }
}

server {
    listen 80;
    server_name forgejo.example.com;
    return 301 https://$host$request_uri;
}
```

Enable and reload:
```sh
ln -s /etc/nginx/sites-available/forgejo /etc/nginx/sites-enabled/forgejo
nginx -t && systemctl reload nginx
```

## 4. Forgejo configuration

In Forgejo's `app.ini`:
```ini
[server]
DOMAIN           = forgejo.example.com
ROOT_URL         = https://forgejo.example.com/
HTTP_ADDR        = 127.0.0.1
HTTP_PORT        = 3000
PROTOCOL         = http
SSH_DOMAIN       = forgejo.example.com
SSH_PORT         = 22
```

Restart Forgejo after editing.

## 5. SSH exposure (optional, for git over SSH)

If git-over-SSH should be public:
- Expose TCP 22 on the edge host (or a dedicated SSH port).
- Forward to the Forgejo node's SSH service (port 22) via the mesh/VPN.
- Keep key-based auth only; disable password auth.

Alternatively, keep SSH private and use HTTPS for public clones.

## 6. Verification

1. `curl -I https://forgejo.example.com` returns 200/302.
2. Clone over HTTPS: `git clone https://forgejo.example.com/jeanterre13/CYNIC.git`.
3. Login flow works; 2FA enabled for admins.
4. Firewall: `nmap` from outside shows only 443 (and 22 if exposed).

## 7. Security notes

- Never expose Forgejo backend port (3000) directly.
- Enforce HTTPS with HSTS.
- Keep Forgejo on the private mesh for admin operations.
- Rotate credentials and review access logs regularly.
- Mutation requires read-back proof (CYNIC invariant); exposure changes must be
  recorded in `infra/` and audited.
EOF
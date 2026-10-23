#cloud-config
hostname: ${VM_NAME}
users:
  - name: ${ADMIN_USER}
    groups: [sudo, adm]
    shell: /bin/bash
    sudo: ALL=(ALL) NOPASSWD:ALL
packages:
  - qemu-guest-agent
  - wireguard
runcmd:
  - systemctl enable qemu-guest-agent
  - systemctl start qemu-guest-agent

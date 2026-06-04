#!/bin/bash
# provision_cynic.sh
# À exécuter en tant que root sur cynic-forge

if [[ $EUID -ne 0 ]]; then
   echo "Ce script doit être exécuté en tant que root"
   exit 1
fi

echo "--- Provisionnement de l'utilisateur 'cynic' ---"

# 1. Création utilisateur
if id "cynic" &>/dev/null; then
    echo "L'utilisateur 'cynic' existe déjà."
else
    useradd -m -s /bin/bash cynic
    echo "Utilisateur 'cynic' créé."
fi

# 2. Groupe Proxmox (pveadmin)
usermod -aG pveadmin cynic
echo "Utilisateur ajouté au groupe 'pveadmin'."

# 3. Setup SSH (répertoire)
mkdir -p /home/cynic/.ssh
chmod 700 /home/cynic/.ssh
chown -R cynic:cynic /home/cynic/.ssh
echo "Répertoire SSH configuré."

# 4. Sudoers pour pvesh
echo "cynic ALL=(ALL) NOPASSWD: /usr/bin/pvesh" > /etc/sudoers.d/cynic
chmod 440 /etc/sudoers.d/cynic
echo "Permissions sudo configurées pour pvesh."

echo "--- Provisionnement terminé ---"
echo "ATTENTION : N'oubliez pas d'ajouter votre clé publique dans /home/cynic/.ssh/authorized_keys"

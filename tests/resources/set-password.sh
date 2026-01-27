#!/bin/bash
echo "Setting up qBittorrent config..."
mkdir -p /config/qBittorrent
cat > /config/qBittorrent/qBittorrent.conf <<EOF
[Preferences]
WebUI\Username=admin
WebUI\Password_PBKDF2="@ByteArray(ARQ77eY1NUZaQsuDHbIMCA==:0WMRkYTUWVT9wVvdDtHAjU9b3b7uB8NR1Gur2hmQCvCDpm39Q+PsJRJPaCU51gEi+f+qlSPa9/ScnBudM17yYA==)"
WebUI\Port=8080
WebUI\Address=*
WebUI\LocalHostAuth=false
WebUI\AuthSubnetWhitelist=0.0.0.0/0
WebUI\AuthSubnetWhitelistEnabled=true
WebUI\HostHeaderValidation=false
WebUI\CSRFProtection=false
LegalNotice\Accepted=true
EOF
chown -R abc:abc /config/qBittorrent
echo "Config written."

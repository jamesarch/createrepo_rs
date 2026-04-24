#!/bin/bash
set -e

echo "=== Installing dnf-plugins-core ==="
dnf install -y dnf-plugins-core 2>&1 | tail -3

echo "=== Configuring RPM repository ==="
cat > /etc/yum.repos.d/test-repo.repo << 'REPOEOF'
[test-repo]
name=Test RPM Repository
baseurl=http://repo-server/
enabled=1
gpgcheck=0
REPOEOF

echo "=== Waiting for nginx ==="
for i in $(seq 1 30); do
  if curl -s http://repo-server/repodata/repomd.xml > /dev/null 2>&1; then
    echo "Repository ready!"
    break
  fi
  echo "Waiting... ($i/30)"
  sleep 2
done

echo ""
echo "=== repomd.xml ==="
curl -s http://repo-server/repodata/repomd.xml
echo ""

echo ""
echo "=== dnf repolist ==="
dnf repolist
echo ""

echo "=== Available packages ==="
dnf --disablerepo='*' --enablerepo='test-repo' list available
echo ""

echo "=== Downloading fake_bash ==="
if dnf --disablerepo='*' --enablerepo='test-repo' download fake_bash --downloaddir=/tmp/ 2>&1; then
  echo ""
  echo "✅✅✅ SUCCESS! createrepo_rs metadata works with dnf!"
  ls -la /tmp/fake_bash*.rpm
else
  echo ""
  echo "❌ Download failed, diagnosing..."
  echo ""
  echo "=== primary.xml (first 30 lines) ==="
  curl -s http://repo-server/repodata/primary.xml.gz | gzip -dc | head -30
fi

echo ""
echo "Container is running. Press Ctrl+C to stop..."
sleep infinity

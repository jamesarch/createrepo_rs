#!/bin/bash
set -e

echo "=== 安装 dnf-plugins-core (提供 download 命令) ==="
dnf install -y dnf-plugins-core 2>&1 | tail -3

echo "=== 配置 RPM 仓库 ==="
cat > /etc/yum.repos.d/test-repo.repo << 'REPOEOF'
[test-repo]
name=Test RPM Repository
baseurl=http://repo-server/
enabled=1
gpgcheck=0
REPOEOF

echo "=== 等待 nginx 就绪 ==="
for i in $(seq 1 30); do
  if curl -s http://repo-server/repodata/repomd.xml > /dev/null 2>&1; then
    echo "仓库就绪！"
    break
  fi
  echo "等待中... ($i/30)"
  sleep 2
done

echo ""
echo "=== repomd.xml ==="
curl -s http://repo-server/repodata/repomd.xml
echo ""

echo ""
echo "=== dnf 仓库列表 ==="
dnf repolist
echo ""

echo "=== 搜索可用包 ==="
dnf --disablerepo='*' --enablerepo='test-repo' list available
echo ""

echo "=== 尝试下载 fake_bash ==="
if dnf --disablerepo='*' --enablerepo='test-repo' download fake_bash --downloaddir=/tmp/ 2>&1; then
  echo ""
  echo "✅✅✅ 成功！Rust createrepo_rs 生成的仓库被 dnf 正常识别并下载！"
  ls -la /tmp/fake_bash*.rpm
else
  echo ""
  echo "❌ 下载失败，诊断中..."
  echo ""
  echo "=== primary.xml 前 30 行 ==="
  curl -s http://repo-server/repodata/primary.xml.gz | gzip -dc | head -30
fi

echo ""
echo "保持运行，可按 Ctrl+C 退出..."
sleep infinity

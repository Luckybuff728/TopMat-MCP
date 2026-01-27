# 🚀 TopMat-LLM 自动化部署指南

本文档详细说明了如何构建、部署并将 TopMat-LLM 服务推送到私有镜像仓库。

## 📋 前置条件

### 1. Docker 配置
由于我们使用的是私有镜像仓库 (`192.168.7.102:5000`)，需要在 Docker Daemon 配置中添加不安全注册表信任。

修改 Docker 配置文件 (`daemon.json`)：
- **Windows**: `C:\Users\<YourUser>\.docker\daemon.json` 或通过 Docker Desktop 设置 -> Docker Engine
- **Linux**: `/etc/docker/daemon.json`

添加以下内容：
```json
{
  "insecure-registries": [
    "192.168.7.102:5000"
  ]
}
```
配置完成后，请重启 Docker 服务。

## 🛠️ 部署流程

我们提供了 Windows 和 Linux 的一键部署脚本，您可以直接复制使用或运行 `scripts/` 目录下的文件。

### Windows (PowerShell) - `scripts/deploy.ps1`
```powershell
# TopMat-LLM Deployment Script

$RegistryUrl = "192.168.7.102:5000"
$ImageName = "topmat-llm"
$Version = "v1.0"
$LatestTag = "$RegistryUrl/$ImageName`:$Version"
$VersionTag = "$RegistryUrl/$ImageName`:latest"

Write-Host "🚀 Starting TopMat-LLM Deployment..." -ForegroundColor Green

# 1. Build Docker Image
Write-Host "`n🔨 Building Docker Image..." -ForegroundColor Cyan
docker build -t $LatestTag -t $VersionTag .
if ($LASTEXITCODE -ne 0) {
    Write-Error "Docker build failed!"
    exit 1
}

# 2. Start Services with Docker Compose
Write-Host "`n🐳 Starting Services..." -ForegroundColor Cyan
docker-compose up -d
if ($LASTEXITCODE -ne 0) {
    Write-Error "Docker compose up failed!"
    exit 1
}

# 3. Push to Registry
Write-Host "`n⬆️ Pushing images to registry ($RegistryUrl)..." -ForegroundColor Cyan
docker push $LatestTag
docker push $VersionTag

if ($LASTEXITCODE -ne 0) {
    Write-Warning "Docker push failed. Check 'insecure-registries' config."
} else {
    Write-Host "✅ Images pushed successfully!" -ForegroundColor Green
}

# 4. Show Logs (Optional, 5 seconds)
Write-Host "`n📋 Tailing logs for 5 seconds..." -ForegroundColor Cyan
$p = Start-Process docker-compose -ArgumentList "logs", "-f", "topmat-llm" -PassThru
Start-Sleep -Seconds 5
Stop-Process -Id $p.Id -ErrorAction SilentlyContinue

Write-Host "`n✨ Deployment Complete! You can view full logs with: docker-compose logs -f topmat-llm" -ForegroundColor Green
```

### Linux (Bash) - `scripts/deploy.sh`
```bash
#!/bin/bash

# TopMat-LLM Deployment Script

REGISTRY_URL="192.168.7.102:5000"
IMAGE_NAME="topmat-llm"
VERSION="v1.0"
LATEST_TAG="$REGISTRY_URL/$IMAGE_NAME:latest"
VERSION_TAG="$REGISTRY_URL/$IMAGE_NAME:$VERSION"

echo -e "\033[0;32m🚀 Starting TopMat-LLM Deployment...\033[0m"

# 1. Build Docker Image
echo -e "\n\033[0;36m🔨 Building Docker Image...\033[0m"
docker build -t $LATEST_TAG -t $VERSION_TAG .
if [ $? -ne 0 ]; then
    echo "Docker build failed!"
    exit 1
fi

# 2. Start Services with Docker Compose
echo -e "\n\033[0;36m🐳 Starting Services...\033[0m"
docker-compose up -d
if [ $? -ne 0 ]; then
    echo "Docker compose up failed!"
    exit 1
fi

# 3. Push to Registry
echo -e "\n\033[0;36m⬆️ Pushing images to registry ($REGISTRY_URL)...\033[0m"
docker push $LATEST_TAG
docker push $VERSION_TAG

if [ $? -ne 0 ]; then
    echo -e "\033[0;33mDocker push failed. Check 'insecure-registries' config.\033[0m"
else
    echo -e "\033[0;32m✅ Images pushed successfully!\033[0m"
fi

# 4. Show Logs (Optional timeout)
echo -e "\n\033[0;36m📋 Tailing logs (Ctrl+C to stop)...\033[0m"
# Not doing timeout here as it is interactive mostly
timeout 5s docker-compose logs -f topmat-llm || true

echo -e "\n\033[0;32m✨ Deployment Complete! View logs with: docker-compose logs -f topmat-llm\033[0m"
```

---

## 📝 手动部署步骤

如果您希望手动执行每一步，请参考以下命令：

### 1. 构建镜像
构建 Docker 镜像并打上 latest 和版本号标签。

```bash
docker build -t 192.168.7.102:5000/topmat-llm:latest -t 192.168.7.102:5000/topmat-llm:v1.0 .
```

### 2. 启动服务
使用 Docker Compose 在后台启动服务。

```bash
docker-compose up -d
```

### 3. 查看日志
查看服务的实时日志以确保启动成功。

```bash
docker-compose logs -f topmat-llm
```

### 4. 推送到仓库
将构建好的镜像推送到私有仓库，以便在其他机器上拉取。

```bash
docker push 192.168.7.102:5000/topmat-llm:latest
docker push 192.168.7.102:5000/topmat-llm:v1.0
```


## 🔄 自动更新配置 (Watchtower)

远程服务器已配置 Watchtower，每隔 1 小时会自动检测并更新镜像。为了启用此功能，我们已在 `docker-compose.yml` 中为服务添加了特定标签：

```yaml
services:
  topmat-llm:
    # ...
    labels:
      - "com.centurylinklabs.watchtower.enable=true"
    # ...
```

只要确保推送到仓库的 `latest` 标签是最新的，远程服务器就会自动拉取并重启服务。

## ⚠️ 常见问题

1. **推送失败**：请检查是否已正确配置 `insecure-registries`。
2. **连接超时**：请确保您可以 ping 通 `192.168.7.102`，并且端口 `5000` 开放。

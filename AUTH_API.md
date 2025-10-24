# TopMat-LLM 鉴权API文档

## 概述

TopMat-LLM 现在支持基于API Key的鉴权机制。鉴权只在连接时进行一次，鉴权通过后可以使用所有聊天功能而无需重复鉴权。

## 鉴权流程

### 1. 连接鉴权

在开始使用聊天功能之前，需要先调用鉴权端点验证API Key：

```bash
curl -X POST http://localhost:3000/auth \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your_api_key_here"
```

**支持的API Key传递方式：**
- Authorization header: `Bearer <api_key>`
- X-API-Key header: `<api_key>`

### 2. 鉴权成功响应

```json
{
  "status": "success",
  "message": "鉴权成功",
  "user": {
    "username": "johndoe",
    "subscription_level": "pro",
    "email": "john.doe@example.com"
  },
  "api_key": {
    "key_name": "Production API Key",
    "expires_at": "2025-12-31T23:59:59Z"
  },
  "timestamp": "2024-10-23T10:30:00Z"
}
```

### 3. 鉴权失败响应

```json
{
  "error": "invalid_api_key",
  "message": "无效的API Key",
  "details": {
    "auth_error": "无效的API Key"
  },
  "timestamp": "2024-10-23T10:30:00Z"
}
```

## 鉴权错误类型

| 错误类型 | HTTP状态码 | 描述 |
|---------|-----------|------|
| `missing_api_key` | 401 | 请求中缺少API Key |
| `invalid_api_key` | 401 | API Key无效 |
| `expired_api_key` | 401 | API Key已过期 |
| `inactive_api_key` | 403 | API Key未激活 |
| `subscription_expired` | 403 | 用户订阅已过期 |
| `auth_service_error` | 503 | 鉴权服务暂时不可用 |

## 使用聊天功能

鉴权通过后，可以直接使用聊天端点：

```bash
curl -X POST http://localhost:3000/chat \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your_api_key_here" \
  -d '{
    "message": "你好，请介绍一下Rust语言",
    "stream": false,
    "model": "qwen-plus"
  }'
```

**注意：** 当前实现中，聊天端点仍会检查API Key的存在性，但不会重复调用外部鉴权服务。

## 配置要求

### 环境变量

```bash
# 鉴权服务地址（可选，默认为 https://api.topmaterial-tech.com）
AUTH_API_URL=https://api.topmaterial-tech.com

# 服务器配置
SERVER_HOST=127.0.0.1
SERVER_PORT=3000
RUST_LOG=info
```

## API Key验证流程

1. **API Key提取** - 从请求的多个位置中提取API Key
2. **外部验证** - 调用 `https://api.topmaterial-tech.com/api/v1/apikey_info` 验证
3. **状态检查** - 检查API Key是否激活、是否过期
4. **订阅检查** - 检查用户订阅是否过期
5. **返回结果** - 鉴权成功返回用户信息，失败返回错误信息

## 安全特性

- **HTTPS传输** - 生产环境建议使用HTTPS
- **API Key掩码** - 日志中只显示API Key前8位
- **详细错误** - 提供明确的错误类型和描述
- **时间戳记录** - 所有请求都包含时间戳

## 故障排除

### 常见问题

1. **401 Unauthorized** - 检查API Key是否正确
2. **403 Forbidden** - 检查API Key是否激活，订阅是否过期
3. **503 Service Unavailable** - 鉴权服务暂时不可用，请稍后重试

### 调试建议

- 检查环境变量配置
- 确认API Key格式正确
- 查看服务器日志获取详细错误信息
- 使用网络工具检查外部API服务状态
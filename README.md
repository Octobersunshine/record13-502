# Location Tracker - Rust + Axum 设备定位数据服务

一个基于 Rust + Axum 构建的高性能后端服务，用于接收设备离线缓存的定位数据包，解析并存储轨迹点。

## 功能特性

- ✅ **多格式数据解析**：支持 JSON、Binary、Hex、CSV、Protobuf 五种数据格式
- ✅ **数据完整性校验**：支持 SHA256 校验和验证
- ✅ **持久化存储**：使用 SQLite 数据库存储原始数据包和解析后的轨迹点
- ✅ **RESTful API**：完整的 CRUD 操作接口
- ✅ **OpenAPI 文档**：集成 Swagger UI 自动生成 API 文档
- ✅ **CORS 支持**：内置跨域资源共享支持
- ✅ **结构化日志**：使用 tracing 进行日志记录
- ✅ **完整错误处理**：统一的错误响应格式

## 技术栈

| 技术 | 版本 | 说明 |
|------|------|------|
| Rust | 1.70+ | 编程语言 |
| Axum | 0.7 | Web 框架 |
| Tokio | 1.x | 异步运行时 |
| SQLx | 0.7 | 数据库 ORM |
| SQLite | 3.x | 数据库 |
| Serde | 1.x | 序列化/反序列化 |
| Utoipa | 4.x | OpenAPI 文档生成 |
| Tracing | 0.1 | 日志框架 |

## 项目结构

```
location-tracker/
├── src/
│   ├── main.rs          # 程序入口，路由配置
│   ├── models.rs        # 数据模型定义
│   ├── db.rs            # 数据库操作层
│   ├── parser.rs        # 数据包解析器
│   ├── handlers.rs      # API 请求处理器
│   └── errors.rs        # 错误类型定义
├── tests/
│   └── integration_test.rs  # 集成测试
├── examples/
│   └── client_example.rs    # 客户端调用示例
├── Cargo.toml           # 项目依赖配置
└── README.md            # 项目文档
```

## 快速开始

### 环境要求

- Rust 1.70+ (建议使用 rustup 安装)
- Windows: 需要安装 Visual Studio Build Tools 或使用 GNU 工具链
  ```powershell
  # 使用 GNU 工具链
  rustup install stable-x86_64-pc-windows-gnu
  rustup default stable-x86_64-pc-windows-gnu
  ```

### 编译运行

```bash
# 编译项目
cargo build --release

# 运行服务
cargo run --release
```

服务默认在 `http://0.0.0.0:3000` 启动。

### 环境变量

| 变量名 | 默认值 | 说明 |
|--------|--------|------|
| `DATABASE_URL` | `sqlite:location.db` | 数据库连接地址 |
| `HOST` | `0.0.0.0` | 服务监听地址 |
| `PORT` | `3000` | 服务监听端口 |

## API 文档

启动服务后，访问 Swagger UI 查看完整的 API 文档：

```
http://localhost:3000/swagger-ui
```

OpenAPI JSON 规范：

```
http://localhost:3000/api-docs/openapi.json
```

## API 接口说明

### 1. 健康检查

**GET** `/health`

```bash
curl http://localhost:3000/health
```

响应示例：
```json
{
  "status": "ok",
  "version": "0.1.0"
}
```

### 2. 上传定位数据

**POST** `/api/v1/location/upload`

支持多种数据格式，请求体结构：

```json
{
  "device_id": "dev_001",
  "device_type": "GPS_TRACKER_V2",
  "firmware_version": "1.0.0",
  "upload_time": "2024-01-15T10:30:00Z",
  "data_format": "json",
  "payload": "[...]",
  "checksum": "optional_sha256_hash",
  "signature": "optional_signature"
}
```

#### 数据格式说明

##### JSON 格式 (`data_format: "json"`)

payload 为 JSON 数组或包含 `points`/`data` 字段的对象：

```json
[
  {
    "latitude": 39.9042,
    "longitude": 116.4074,
    "timestamp": "2024-01-15T10:25:30Z",
    "altitude": 45.5,
    "speed": 60.0,
    "heading": 180.0,
    "satellites": 8,
    "source": "GPS"
  }
]
```

支持的字段名变体：
- 纬度：`latitude`, `lat`
- 经度：`longitude`, `lng`, `lon`
- 时间：`timestamp`, `time`, `t` (支持 RFC3339 或 Unix 时间戳)
- 高度：`altitude`, `alt`
- 速度：`speed`, `spd`
- 方向：`heading`, `dir`, `bearing`
- 卫星数：`satellites`, `sat`

##### 二进制格式 (`data_format: "binary"`)

payload 为 Base64 编码的二进制数据，每个轨迹点 16 字节：

| 偏移 | 长度 | 类型 | 说明 |
|------|------|------|------|
| 0 | 4 | u32 LE | Unix 时间戳 |
| 4 | 4 | i32 LE | 纬度 (×1,000,000) |
| 8 | 4 | i32 LE | 经度 (×1,000,000) |
| 12 | 2 | i16 LE | 高度 (×10) |
| 14 | 1 | u8 | 速度 (×0.5, km/h) |
| 15 | 1 | u8 | 方向 (×1.4117647, 度) |
| 16 | 1 | u8 | 卫星数量 |
| 17 | 1 | u8 | 标志位 |

标志位定义：
- Bit 0: GPS 定位
- Bit 1: LBS 定位
- Bit 2: WIFI 定位
- Bit 3: 包含电量信息

##### Hex 格式 (`data_format: "hex"`)

payload 为十六进制字符串，二进制结构同上。

##### CSV 格式 (`data_format: "csv"`)

payload 为 CSV 格式数据，首行为表头：

```csv
latitude,longitude,timestamp,altitude,speed,satellites
39.9042,116.4074,2024-01-15T10:25:30Z,45.5,60.0,8
```

##### Protobuf 格式 (`data_format: "protobuf"`)

payload 为 Base64 编码的 Protocol Buffer 数据。

### 3. 查询轨迹点

**GET** `/api/v1/track/points`

查询参数：
- `device_id`: 设备 ID（可选）
- `start_time`: 开始时间，RFC3339 格式（可选）
- `end_time`: 结束时间，RFC3339 格式（可选）
- `limit`: 返回数量限制，默认 100（可选）
- `offset`: 偏移量（可选）

```bash
# 查询指定设备的轨迹点
curl "http://localhost:3000/api/v1/track/points?device_id=dev_001&limit=10"

# 按时间范围查询
curl "http://localhost:3000/api/v1/track/points?start_time=2024-01-01T00:00:00Z&end_time=2024-01-02T00:00:00Z"
```

### 4. 获取单个轨迹点

**GET** `/api/v1/track/points/{id}`

```bash
curl "http://localhost:3000/api/v1/track/points/550e8400-e29b-41d4-a716-446655440000"
```

### 5. 删除轨迹点

**DELETE** `/api/v1/track/points/{id}`

```bash
curl -X DELETE "http://localhost:3000/api/v1/track/points/550e8400-e29b-41d4-a716-446655440000"
```

### 6. 获取设备列表

**GET** `/api/v1/devices`

```bash
curl "http://localhost:3000/api/v1/devices"
```

### 7. 获取原始数据包

**GET** `/api/v1/packets/{id}`

```bash
curl "http://localhost:3000/api/v1/packets/550e8400-e29b-41d4-a716-446655440000"
```

## 数据库结构

### `raw_packets` 表 - 原始数据包

| 字段 | 类型 | 说明 |
|------|------|------|
| `id` | TEXT | UUID 主键 |
| `device_id` | TEXT | 设备 ID |
| `device_type` | TEXT | 设备类型 |
| `firmware_version` | TEXT | 固件版本 |
| `upload_time` | TEXT | 上传时间 |
| `data_format` | TEXT | 数据格式 |
| `payload` | TEXT | 原始数据载荷 |
| `checksum` | TEXT | 校验和 |
| `signature` | TEXT | 签名 |
| `parsed` | INTEGER | 是否已解析 |
| `parse_error` | TEXT | 解析错误信息 |
| `created_at` | TEXT | 创建时间 |

### `track_points` 表 - 轨迹点

| 字段 | 类型 | 说明 |
|------|------|------|
| `id` | TEXT | UUID 主键 |
| `device_id` | TEXT | 设备 ID |
| `latitude` | REAL | 纬度 |
| `longitude` | REAL | 经度 |
| `altitude` | REAL | 高度 (米) |
| `speed` | REAL | 速度 (km/h) |
| `heading` | REAL | 方向 (度) |
| `satellites` | INTEGER | 卫星数量 |
| `hdop` | REAL | 水平精度因子 |
| `timestamp` | TEXT | 定位时间 |
| `location_source` | TEXT | 定位来源 (GPS/LBS/WIFI) |
| `accuracy` | REAL | 精度 (米) |
| `battery_level` | REAL | 电量 (%) |
| `extra_data` | TEXT | 额外数据 (JSON) |
| `raw_packet_id` | TEXT | 关联原始数据包 ID |
| `created_at` | TEXT | 创建时间 |

## 测试

```bash
# 运行单元测试和集成测试
cargo test

# 运行客户端示例（需要先启动服务）
cargo run --example client_example
```

## 客户端调用示例

参考 [`examples/client_example.rs`](examples/client_example.rs) 查看完整的 API 调用示例，包括：

- 上传 JSON 格式数据
- 上传 Binary 格式数据
- 上传 CSV 格式数据
- 带校验和的数据上传
- 查询轨迹点
- 获取设备列表

## 性能优化建议

1. **数据库索引**：已为 `device_id`、`timestamp` 创建复合索引
2. **批量插入**：支持一次性解析并存储多个轨迹点
3. **连接池**：使用 SQLx 连接池管理数据库连接
4. **异步处理**：全异步 I/O 操作，高并发支持

## 安全建议

1. **API 认证**：建议添加 API Key 或 JWT 认证
2. **HTTPS**：生产环境启用 HTTPS
3. **请求限流**：添加速率限制防止滥用
4. **输入验证**：已实现基本的输入验证，建议根据业务需求增强
5. **数据加密**：敏感数据建议加密存储

## 常见问题

### Q: 如何添加新的数据格式支持？

A: 在 `parser.rs` 中添加新的解析函数，并在 `DataFormat` 枚举中添加新类型。

### Q: 如何更换数据库？

A: 修改 `Cargo.toml` 中的 SQLx 特性，将 `sqlite` 改为 `postgres` 或 `mysql`，并调整 `db.rs` 中的 SQL 语法。

### Q: 如何处理超大数据包？

A: 当前实现会先存储原始数据包再解析。建议在反向代理层配置请求体大小限制。

## License

MIT

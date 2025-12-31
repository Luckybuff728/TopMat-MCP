# TopMat-LLM Dockerfile
# 多阶段构建，优化镜像大小

# 阶段 1: 构建环境
FROM rust:1.88-slim as builder

# 安装系统依赖
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    ca-certificates \
    sqlite3 \
    && rm -rf /var/lib/apt/lists/*

# 设置工作目录
WORKDIR /app

# 复制构建文件（优化缓存顺序）
COPY Cargo.toml Cargo.lock ./

# 复制源代码
COPY src ./src
COPY rig ./rig
# 构建应用程序
RUN cargo build --release --locked

# 阶段 2: 运行时环境
FROM debian:bookworm-slim



# 设置工作目录
WORKDIR /app

# 从构建阶段复制二进制文件
COPY --from=builder /app/target/release/TopMat-LLM /app/TopMat-LLM
# 安装运行时依赖
RUN apt-get update && apt-get install -y \
    ca-certificates \
    sqlite3 \
    tzdata \
    && ln -snf /usr/share/zoneinfo/Asia/Shanghai /etc/localtime \
    && echo "Asia/Shanghai" > /etc/timezone \
    && rm -rf /var/lib/apt/lists/*


# 暴露端口
EXPOSE 3000

# 环境变量
ENV RUST_LOG=info
ENV DATABASE_URL=sqlite:/app/data/data.db

# 启动命令
ENTRYPOINT ["./TopMat-LLM"]
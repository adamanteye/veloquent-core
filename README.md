# Veloquent 后端服务

队名 **Veloquent** 结合拉丁语 _velox_ (快速) 和 _eloquent_ (雄辩), 表达快速而清晰的沟通能力.

数据库采用 `postgres:9.5.20-alpine`.

**Veloquent** 后端服务的配置文件示例为 `veloquent.toml`, 包含数据库敏感信息.

## 部署说明

因为 secoder 不支持环境变量运行容器, 而且对于高版本的 postgresql 无法使用终端, 因此使用镜像 `postgres:9.5.20-alpine`. 为了配置用户名和数据库, 首先连入 web 终端:

```sh
psql -U postgres
CREATE USER yangzheh WITH PASSWORD '123456';
CREATE DATABASE veloquent;
GRANT ALL PRIVILEGES ON DATABASE veloquent TO yangzheh;
\c veloquent
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
\q
```

## 测试说明

运行 llvm-cov 工具:

```sh
cargo llvm-cov --no-clean --ignore-filename-regex=src/entity/.\*
```

在此之前, 可能需要安装该工具:

```sh
cargo install cargo-llvm-cov --version 0.6.14 --locked
```

如果运行时提示缺少 `llvm-tools-preview`, 安装:

```sh
rustup component add llvm-tools-preview
```

## 调试说明

在本地开发过程中, 在后台启动一个 Postgres 数据库:

```sh
docker run -d \
    --name pg-dev \
    -e POSTGRES_USER=yangzheh \
    -e POSTGRES_PASSWORD=123456 \
    -v /srv/pg-dev:/var/lib/postgresql/data \
    -p 5432:5432 \
    postgres:9.5.20-alpine
```

如果想连入数据库执行一些操作, 可以执行:

```sh
docker exec -it pg-dev psql -U yangzheh
```

完成新的 migration 后, 通过下列命令生成 entity:

```sh
sea-orm-cli generate entity \
    -u postgres://yangzheh:123456@localhost:5432/veloquent \
    -o src/entity
```

如果想导出 SQL 文件, 可以执行:

```sh
docker exec -it pg-dev pg_dump veloquent > velo.sql -U yangzheh
```

## API 文档

由 utoipa 生成, 路径在后端服务的 `/doc` 下.

## 代码文档

运行

```sh
cargo doc --no-deps
```

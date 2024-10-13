# Veloquent 后端服务

队名 **Veloquent** 结合拉丁语 _velox_(快速) 和 _eloquent_ (雄辩), 表达快速而清晰的沟通能力.

数据库采用 `postgres:17.0-alpine3.20`, 详见 `docker-compose.yml`.

**Veloquent** 后端服务的配置文件示例为 `veloquent.toml`, 包含数据库敏感信息.

## 测试说明

运行 llvm-cov 工具
```sh
cargo llvm-cov
```

## 调试说明

在本地开发过程中, 在后台启动一个 Postgres 数据库可以如此进行.

```sh
docker run -d \
    --name pg-dev \
    -e POSTGRES_USER=yangzheh \
    -e POSTGRES_PASSWORD=123456 \
    -v /srv/pg-dev:/var/lib/postgresql/data \
    -p 5432:5432 \
    postgres:17.0-alpine3.20
```

如果想连入数据库执行一些操作, 可以执行

```sh
docker exec -it pg-dev psql -U yangzheh
```

完成新的 migration 后, 通过下列命令生成 entity.

```sh
sea-orm-cli generate entity \
    -u postgres://yangzheh:123456@localhost:5432/veloquent \
    -o src/entity
```

## API 文档

由 utoipa 生成, 路径在后端服务的 `/doc` 下.

## 代码文档

运行

```sh
argo doc --no-deps
```
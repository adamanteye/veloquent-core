# Veloquent 后端服务

队名 **Veloquent** 结合拉丁语 _velox_(快速) 和 _eloquent_ (雄辩), 表达快速而清晰的沟通能力.

数据库采用 `postgres:17.0-alpine3.20`, 详见 `docker-compose.yml`.

**Velogent** 后端服务的配置文件示例为 `veloquent.toml`, 包含数据库敏感信息.

## API 文档

由 utoipa 生成, 路径在后端服务的 `/doc` 下.

## 代码文档

运行

```sh
argo doc --no-deps
```

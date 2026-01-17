# simple-http

A simple HTTP server that counts requests by IP address and path.

## Features

- Request counting by IP + path combination
- Cloudflare support (uses `cf-connecting-ip` header when present)
- Configurable listen address via CLI
- Health check endpoint

## Endpoints

| Endpoint | Description |
|----------|-------------|
| `/healthz` | Health check, returns `OK` |
| `/*path` | Returns JSON with request count for the current IP + path |

### Example Response

```json
{
  "total": 5,
  "ip": "192.168.1.1",
  "path": "/api/users"
}
```

## Usage

### Running locally

```bash
cargo run -- --address 0.0.0.0:8080
```

### CLI Options

| Option | Short | Default | Description |
|--------|-------|---------|-------------|
| `--address` | `-a` | `0.0.0.0:8080` | Listen address |

### Docker

```bash
docker pull ghcr.io/mipsel64/simple-http:latest
docker run -p 8080:8080 ghcr.io/mipsel64/simple-http:latest
```

With custom address:

```bash
docker run -p 3000:3000 ghcr.io/mipsel64/simple-http:latest --address 0.0.0.0:3000
```

## Building

```bash
cargo build --release
```

## License

[MIT](./LICENSE)

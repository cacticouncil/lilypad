# Lilypad's Telemetry Server

## Local Dev

### Node

1. Install node
2. `npm install`
3. `npm run dev`

### Database

1. Install Postgres
2. Start Postgres (machine dependent)
   - macOS with Homebrew: `postgres -D /opt/homebrew/var/postgresql@14`
3. Create database: `createdb LilypadTelemetry`
4. Add `DATABASE_URL` to `.env`

### `.env` Contents

```env
PORT='8000'
DB_HOST='localhost`
DB_NAME='LilypadTelemetry'
DB_USER='username of user that ran createdb'
DB_PASSWORD='password of user that ran createdb'
DB_PORT='5432'
```

## Docker Dev

1. [Install Docker](https://docs.docker.com/get-docker/)
2. Create `.env` with everything except `DATABASE_URL` in `/`
3. Run: `docker compose up`

### `.env` Contents

```env
PORT='8000'
DB_NAME='LilypadTelemetry'
DB_USER='docker'
DB_PASSWORD='docker'
```

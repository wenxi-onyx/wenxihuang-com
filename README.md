## Architecture

- **Backend**: Rust + Axum web framework + SQLx (PostgreSQL)
- **Frontend**: SvelteKit + TypeScript + TailwindCSS v4
- **Database**: PostgreSQL on Fly.io
- **Deployment**: Fly.io with Docker containers

## Project Structure

```
.
├── backend/              # Rust/Axum API server
│   ├── src/
│   │   ├── main.rs      # Application entry point
│   │   ├── handlers/    # HTTP request handlers
│   │   ├── middleware/  # Auth & CORS middleware
│   │   ├── models/      # Database models
│   │   ├── services/    # Business logic
│   │   └── bin/         # Utility binaries
│   ├── migrations/      # SQL migration files
│   ├── Cargo.toml       # Rust dependencies
│   └── Dockerfile       # Backend container config
├── frontend/            # SvelteKit application
│   ├── src/
│   │   ├── lib/         # Components & utilities
│   │   └── routes/      # SvelteKit routes
│   ├── package.json     # Node dependencies
│   └── Dockerfile       # Frontend container config
└── README.md           # This file
```

## Prerequisites

- [Rust](https://rustup.rs/) (latest stable)
- [Node.js](https://nodejs.org/) v24+
- [Fly.io CLI](https://fly.io/docs/hands-on/install-flyctl/)
- [PostgreSQL](https://www.postgresql.org/) (for local development)

## Local Development

### 1. Database Setup

**Recommended: Using Docker Compose**

The easiest way to set up PostgreSQL locally:

```bash
# Start PostgreSQL (from project root)
docker compose up -d

# Verify it's running
docker ps | grep wenxihuang-postgres-dev

# View logs
docker compose logs postgres

# Stop when done
docker compose stop

# Start again later
docker compose start
```

**Alternative: Using Docker directly**

```bash
docker run --name wenxihuang-postgres-dev \
  -e POSTGRES_PASSWORD=postgres \
  -e POSTGRES_DB=wenxihuang_dev \
  -p 5433:5432 \
  -d postgres:17
```

**Alternative: System PostgreSQL**

```bash
# Create database on your local PostgreSQL
createdb wenxihuang_dev

# Update backend/.env to use port 5432 instead of 5433
```

**Note**: Local development uses port **5433** instead of 5432 to avoid conflicts with other PostgreSQL instances.

### 2. Backend Setup

```bash
cd backend

# Copy environment template
cp .env.example .env

# Edit .env with your local configuration
# DATABASE_URL=postgresql://postgres:postgres@localhost:5433/wenxihuang_dev
# SESSION_SECRET=your-random-secret-key-here
# PORT=8083

# Run migrations
# (Note: Current implementation runs migrations on app startup via SQLx)

# Start development server
cargo run
```

The backend will be available at `http://localhost:8083`

**Note**: Local development uses port **8083** instead of 8080 to avoid conflicts. Production uses port 8080.

### 3. Frontend Setup

```bash
cd frontend

# Copy environment template
cp .env.example .env

# Edit .env
# VITE_API_URL=http://localhost:8083

# Install dependencies
npm ci

# Start development server
npm run dev
```

The frontend will be available at `http://localhost:5173` (auto-increments to 5174, 5175... if port is taken)

### 4. Create First Admin User

```bash
cd backend

# Generate password hash
cargo run --bin create_admin -- "your-password-here"

# Copy the hash output, then connect to database
psql -d wenxihuang_dev

# Insert admin user
INSERT INTO users (username, password_hash, role)
VALUES ('admin', 'PASTE_HASH_HERE', 'admin');
```

## Deployment to Fly.io

### Initial Setup

1. **Authenticate with Fly.io**:
   ```bash
   flyctl auth login
   ```

2. **Create PostgreSQL Database** (first time only):
   ```bash
   flyctl postgres create \
     --name wenxihuang-db \
     --region sjc \
     --initial-cluster-size 1
   ```

3. **Create Applications** (first time only):
   ```bash
   # Backend
   cd backend
   flyctl launch --name wenxihuang-backend --region sjc --no-deploy

   # Frontend
   cd ../frontend
   flyctl launch --name wenxihuang-frontend --region sjc --no-deploy
   ```

4. **Attach Database to Backend**:
   ```bash
   flyctl postgres attach \
     --app wenxihuang-backend \
     wenxihuang-db
   ```
   This automatically sets the `DATABASE_URL` secret.

5. **Set Environment Secrets**:
   ```bash
   # Backend secrets
   flyctl secrets set \
     SESSION_SECRET=$(openssl rand -base64 32) \
     -a wenxihuang-backend
   ```

   **Note**: Frontend API URL is configured via `fly.toml` build args (see `frontend/fly.toml`), not secrets. The `VITE_API_URL` must be set at build time for Vite to embed it in the compiled JavaScript.

### Deploying Updates

#### Deploy Backend

```bash
cd backend
flyctl deploy
```

This will:
1. Build Docker image with multi-stage Rust compilation (~2-3 min)
2. Push to Fly.io registry
3. Rolling deployment to replace existing machines
4. Health checks ensure successful deployment

#### Deploy Frontend

```bash
cd frontend
flyctl deploy
```

This will:
1. Build SvelteKit app with Vite (VITE_API_URL from fly.toml build args)
2. Pre-render static pages (landing page)
3. Create production Node.js server
4. Deploy to Fly.io

**Important**:
- Landing page is pre-rendered at build time for better performance
- Login page uses client-side rendering (SSR disabled for auth)
- API URL is baked into the build via `VITE_API_URL` build arg in `fly.toml`

### Deployment Verification

```bash
# Check backend health
curl https://wenxihuang-backend.fly.dev/health

# Check frontend
curl https://wenxihuang-frontend.fly.dev/

# View logs
flyctl logs -a wenxihuang-backend
flyctl logs -a wenxihuang-frontend

# Check app status
flyctl status -a wenxihuang-backend
flyctl status -a wenxihuang-frontend
```

## Database Operations

### Connecting to Production Database

#### Interactive SQL Console

```bash
# Connect to PostgreSQL directly
flyctl postgres connect -a wenxihuang-db -d wenxihuang_backend
```

You'll be in a `psql` session where you can run SQL commands:
```sql
-- List tables
\dt

-- View users
SELECT id, username, role, created_at FROM users;

-- View sessions
SELECT id, user_id, expires_at FROM sessions;

-- Exit
\q
```

#### Run SQL from Script

```bash
# Execute SQL file
cat migration.sql | flyctl postgres connect -a wenxihuang-db -d wenxihuang_backend

# Execute inline SQL
cat <<EOF | flyctl postgres connect -a wenxihuang-db -d wenxihuang_backend
SELECT COUNT(*) FROM users;
\q
EOF
```

#### SSH into Database Machine

```bash
flyctl ssh console -a wenxihuang-db
```

### Running Migrations

Migrations are located in `backend/migrations/` and follow the naming pattern:
- `001_create_users_and_sessions.sql`
- `002_create_table_tennis_tables.sql`

#### Manual Migration Execution

```bash
# Run a specific migration
cat backend/migrations/001_create_users_and_sessions.sql | \
  flyctl postgres connect -a wenxihuang-db -d wenxihuang_backend
```

#### Migration Script (Recommended)

Create a migration runner script:

```bash
#!/bin/bash
# scripts/run-migrations.sh

MIGRATIONS_DIR="backend/migrations"
DB_APP="wenxihuang-db"
DB_NAME="wenxihuang_backend"

for file in "$MIGRATIONS_DIR"/*.sql; do
  echo "Running migration: $(basename $file)"
  cat "$file" | flyctl postgres connect -a "$DB_APP" -d "$DB_NAME"
done

echo "All migrations completed!"
```

Make it executable and run:
```bash
chmod +x scripts/run-migrations.sh
./scripts/run-migrations.sh
```

### Creating First Admin User in Production

```bash
# 1. Generate password hash locally
cd backend
HASH=$(cargo run --bin create_admin -- "your-secure-password")

# 2. Insert into production database
cat <<EOF | flyctl postgres connect -a wenxihuang-db -d wenxihuang_backend
INSERT INTO users (username, password_hash, role)
VALUES ('admin', '$HASH', 'admin');
\q
EOF

# 3. Verify login
curl -X POST https://wenxihuang-backend.fly.dev/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username": "admin", "password": "your-secure-password"}'
```

### Database Maintenance

#### List Database Users

```bash
flyctl postgres users list -a wenxihuang-db
```

#### Create Database Backup

```bash
# Create snapshot
flyctl volumes snapshots create -a wenxihuang-db

# List snapshots
flyctl volumes snapshots list -a wenxihuang-db
```

#### View Database Metrics

```bash
flyctl postgres db list -a wenxihuang-db
```

## Common Tasks

### Viewing Logs

```bash
# Backend logs (no tail, just current)
flyctl logs -a wenxihuang-backend -n

# Frontend logs
flyctl logs -a wenxihuang-frontend -n

# Follow logs in real-time
flyctl logs -a wenxihuang-backend

# Filter logs by machine ID
flyctl logs -a wenxihuang-backend --machine MACHINE_ID
```

### SSH into Application Machines

```bash
# SSH into backend
flyctl ssh console -a wenxihuang-backend

# List files
ls -la /app

# Check binary
./backend --version  # (if implemented)

# Exit
exit
```

### Scaling Applications

```bash
# Scale up backend machines
flyctl scale count 2 -a wenxihuang-backend

# Scale down
flyctl scale count 1 -a wenxihuang-backend

# Increase memory
flyctl scale memory 512 -a wenxihuang-backend
```

### Managing Secrets

```bash
# List current secrets (values not shown)
flyctl secrets list -a wenxihuang-backend

# Set a secret
flyctl secrets set NEW_SECRET=value -a wenxihuang-backend

# Remove a secret
flyctl secrets unset OLD_SECRET -a wenxihuang-backend

# Rotate session secret
flyctl secrets set SESSION_SECRET=$(openssl rand -base64 32) -a wenxihuang-backend
```

### Health Checks

```bash
# Backend health endpoint
curl https://wenxihuang-backend.fly.dev/health

# Expected response:
# {"status":"healthy","timestamp":"2025-11-02T..."}

# Frontend (root)
curl https://wenxihuang-frontend.fly.dev/
```

## Troubleshooting

### Backend Won't Start

```bash
# Check logs for errors
flyctl logs -a wenxihuang-backend -n

# Common issues:
# - DATABASE_URL not set → flyctl secrets list -a wenxihuang-backend
# - Migration failure → Run migrations manually
# - Port binding → Check fly.toml internal_port matches code (8080)
```

### Database Connection Issues

```bash
# Verify database is running
flyctl status -a wenxihuang-db

# Check if backend can connect
flyctl ssh console -a wenxihuang-backend -C "env | grep DATABASE_URL"

# Test connection from backend machine
flyctl ssh console -a wenxihuang-backend
# Inside: ping wenxihuang-db.internal
```

### Migration Conflicts

If migrations were partially applied:

```bash
# Connect to database
flyctl postgres connect -a wenxihuang-db -d wenxihuang_backend

# Check what exists
\dt

# Drop and recreate tables (DESTRUCTIVE!)
DROP TABLE IF EXISTS sessions CASCADE;
DROP TABLE IF EXISTS users CASCADE;
DROP TYPE IF EXISTS user_role CASCADE;
\q

# Re-run migrations
cat backend/migrations/001_create_users_and_sessions.sql | \
  flyctl postgres connect -a wenxihuang-db -d wenxihuang_backend
```

### Frontend Not Connecting to Backend

1. **Check API URL Configuration**:
   The `VITE_API_URL` must be set at build time in `frontend/fly.toml`:
   ```toml
   [build.args]
     VITE_API_URL = "https://wenxihuang-backend.fly.dev"
   ```

   Verify it's embedded in the build:
   ```bash
   flyctl ssh console -a wenxihuang-frontend -C "grep -r 'wenxihuang-backend' /app/build"
   ```

2. **Verify CORS Configuration** in `backend/src/middleware/cors.rs`:
   - Should allow `https://wenxihuang-frontend.fly.dev`
   - Should allow `https://wenxihuang.com` if using custom domain

3. **Check Browser Console** for CORS or network errors

### Login Page Shows "Load Failed"

If you see a "load failed" error on the login page:

1. **Check Prerender Configuration**:
   - Layout should have `prerender = 'auto'` in `src/routes/+layout.ts`
   - Login page should have `prerender = false` in `src/routes/login/+page.ts`

2. **Verify No Loading Blocker**:
   - Layout should not block rendering waiting for auth check
   - Auth check should happen in background (see `src/routes/+layout.svelte`)

3. **Check Build Logs**:
   ```bash
   flyctl logs -a wenxihuang-frontend
   ```
   Look for build errors or warnings about prerendering

4. **Verify API URL in Browser**:
   Open browser dev tools → Network tab → Check if API calls go to correct backend URL

## Development Workflow

### Pre-commit Checks

Pre-commit hooks are configured in `.pre-commit-config.yaml`:

```bash
# Install pre-commit
pip install pre-commit

# Install hooks
pre-commit install

# Run manually
pre-commit run --all-files
```

Checks include:
- Trailing whitespace removal
- Rust formatting (`cargo fmt`)
- Rust linting (`cargo clippy`)
- Cargo check

### Code Quality

```bash
# Backend
cd backend
cargo fmt        # Format code
cargo clippy     # Lint
cargo test       # Run tests (when implemented)

# Frontend
cd frontend
npm run format   # Format with Prettier
npm run lint     # Lint with ESLint
npm run check    # TypeScript type checking
```

## API Endpoints

### Authentication

| Endpoint | Method | Auth Required | Description |
|----------|--------|---------------|-------------|
| `/` | GET | No | API info |
| `/health` | GET | No | Health check |
| `/api/auth/login` | POST | No | Login with username/password |
| `/api/auth/logout` | POST | Yes | Logout current session |
| `/api/auth/me` | GET | Yes | Get current user info |
| `/api/auth/register` | POST | Yes (Admin) | Create new user (admin-only) |

### Request/Response Examples

**Login:**
```bash
curl -X POST https://wenxihuang-backend.fly.dev/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username": "admin", "password": "changeme123"}'

# Response:
# {
#   "user": {
#     "id": "uuid-here",
#     "username": "admin",
#     "role": "Admin"
#   }
# }
# Cookie: session_id=...
```

**Get Current User:**
```bash
curl https://wenxihuang-backend.fly.dev/api/auth/me \
  -H "Cookie: session_id=YOUR_SESSION_ID"
```

## Security Considerations

### Fixed Issues

1. ✅ **Registration Endpoint**: Now protected with admin-only middleware
2. ✅ **Session Secret**: Enforced 32+ character minimum, loaded from environment
3. ✅ **Error Handling**: All panics (`unwrap()`/`expect()`) replaced with proper error handling
4. ✅ **CORS Configuration**: Now environment-based and validated

### Current Issues

1. **Password Policy**: No complexity requirements enforced
2. **Rate Limiting**: Not implemented - vulnerable to brute force attacks
3. **Session Rotation**: Sessions don't have automatic rotation on privilege changes

### Recommended Practices

- Rotate `SESSION_SECRET` regularly
- Use strong passwords for admin accounts
- Monitor logs for suspicious activity
- Keep dependencies updated
- Review security audit in code review documentation

## Production Configuration

### Resource Optimization

Current setup is optimized for minimal resource usage (1-2 concurrent users):

- **Frontend**: 1 machine, 256MB RAM, shared CPU
  - Auto-stop when idle (min_machines_running = 0)
  - Auto-start on request
- **Backend**: 1 machine, 256MB RAM, shared CPU
- **Database**: 1 machine, 256MB RAM, shared CPU

```bash
# View current machine configuration
flyctl machines list -a wenxihuang-frontend
flyctl machines list -a wenxihuang-backend

# Remove extra machines if needed
flyctl machines remove MACHINE_ID -a wenxihuang-frontend --force
```

### Monitoring

Key metrics to watch:

```bash
# Application status
flyctl status -a wenxihuang-backend
flyctl status -a wenxihuang-frontend

# Database disk usage
flyctl volumes list -a wenxihuang-db

# Check machine resource usage
flyctl machine status -a wenxihuang-backend
```

### Alerting

Set up Fly.io monitoring in the dashboard at https://fly.io/dashboard

Monitor:
- Application uptime
- Response times
- Error rates
- Database connections
- Disk usage

## Custom Domain Setup

1. **Configure DNS** (at your domain registrar):
   ```
   ALIAS   @    wenxihuang-frontend.fly.dev
   CNAME   api  wenxihuang-backend.fly.dev
   CNAME   www  wenxihuang-frontend.fly.dev
   ```

2. **Add certificates** (Fly.io handles automatically):
   ```bash
   flyctl certs create wenxihuang.com -a wenxihuang-frontend
   flyctl certs create www.wenxihuang.com -a wenxihuang-frontend
   flyctl certs create api.wenxihuang.com -a wenxihuang-backend
   ```

3. **Update CORS settings** in `backend/src/middleware/cors.rs`

## Backup and Recovery

### Database Backups

```bash
# Manual backup (export to local file)
flyctl postgres connect -a wenxihuang-db -d wenxihuang_backend <<EOF > backup.sql
\copy users TO STDOUT WITH CSV HEADER;
\copy sessions TO STDOUT WITH CSV HEADER;
\q
EOF

# Restore from backup
flyctl postgres connect -a wenxihuang-db -d wenxihuang_backend < backup.sql
```

### Application State

All application state is in PostgreSQL. To fully backup:
1. Export database using pg_dump
2. Store backup in secure location (S3, etc.)
3. Keep multiple backup generations

## Contributing

This is a personal project, but contributions are welcome!

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run pre-commit checks
5. Submit a pull request

## License

Private/Personal Use

## Contact

- Website: https://wenxihuang.com
- Email: mail@wenxihuang.com

---

**Last Updated**: November 3, 2025
**Deployment Platform**: Fly.io
**Current Version**: 0.1.0

## Recent Changes (Nov 3, 2025)

- ✅ Fixed login page "load failed" error (prerender config + VITE_API_URL setup)
- ✅ Enabled landing page prerendering for better performance
- ✅ Removed loading flash on page load
- ✅ Updated local development ports (Postgres: 5433, Backend: 8083)
- ✅ Added docker-compose.yml for easy local setup
- ✅ Optimized production to 1 machine per service
- ✅ Fixed all security issues from code review (admin-only registration, error handling, CORS)

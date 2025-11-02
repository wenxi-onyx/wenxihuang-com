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

Start a local PostgreSQL instance:

```bash
# Using Docker
docker run --name wenxihuang-db \
  -e POSTGRES_PASSWORD=postgres \
  -e POSTGRES_DB=wenxihuang_dev \
  -p 5432:5432 \
  -d postgres:17

# Or use your system PostgreSQL
createdb wenxihuang_dev
```

### 2. Backend Setup

```bash
cd backend

# Copy environment template
cp .env.example .env

# Edit .env with your local database URL
# DATABASE_URL=postgresql://postgres:postgres@localhost:5432/wenxihuang_dev
# SESSION_SECRET=your-random-secret-key-here
# PORT=8080

# Run migrations
# (Note: Current implementation runs migrations on app startup via SQLx)

# Start development server
cargo run
```

The backend will be available at `http://localhost:8080`

### 3. Frontend Setup

```bash
cd frontend

# Copy environment template
cp .env.example .env

# Edit .env
# VITE_API_URL=http://localhost:8080

# Install dependencies
npm ci

# Start development server
npm run dev
```

The frontend will be available at `http://localhost:5173`

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

   # Frontend environment (if needed)
   flyctl secrets set \
     VITE_API_URL=https://wenxihuang-backend.fly.dev \
     -a wenxihuang-frontend
   ```

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
1. Build SvelteKit app with Vite
2. Create production Node.js server
3. Deploy to Fly.io

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
- `002_create_ping_pong_tables.sql`

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

1. Check `VITE_API_URL` environment variable:
   ```bash
   flyctl secrets list -a wenxihuang-frontend
   ```

2. Verify CORS configuration in `backend/src/middleware/cors.rs`:
   - Should allow `https://wenxihuang-frontend.fly.dev`
   - Should allow `https://wenxihuang.com` if using custom domain

3. Check browser console for CORS errors

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
| `/api/auth/register` | POST | No* | Create new user (should be admin-only) |

*Note: Registration currently has no permission check - see Security Considerations

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

### Current Issues

1. **Registration Endpoint**: Currently open to anyone - should require admin authentication
2. **Session Secret**: Must be set via environment variable in production
3. **Password Policy**: No complexity requirements enforced
4. **Rate Limiting**: Not implemented - vulnerable to brute force attacks

### Recommended Practices

- Rotate `SESSION_SECRET` regularly
- Use strong passwords for admin accounts
- Monitor logs for suspicious activity
- Keep dependencies updated
- Review security audit in code review documentation

## Production Monitoring

### Key Metrics to Watch

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

**Last Updated**: November 2, 2025
**Deployment Platform**: Fly.io
**Current Version**: 0.1.0

# Database Migrations

This directory contains consolidated database migrations organized by major system/concept.

## Migration Structure

Each migration represents a complete system or major feature:

### 001: Authentication System
- **File**: `001_create_authentication_system.sql`
- **Purpose**: User authentication with roles and sessions
- **Tables**: `users`, `sessions`
- **Types**: `user_role` enum (admin, user)
- **Features**: User profiles with first_name/last_name

### 002: Table Tennis Core
- **File**: `002_create_table_tennis_core.sql`
- **Purpose**: Core table tennis game tracking
- **Tables**: `players`, `games`, `elo_history`
- **Features**:
  - Player management with active status
  - Game recording
  - ELO history tracking with version support
  - Auto-updated timestamps via trigger

### 003: ELO Configuration System
- **File**: `003_create_elo_configuration_system.sql`
- **Purpose**: Manage different ELO algorithm configurations
- **Tables**: `elo_configurations`
- **Features**:
  - Multiple ELO algorithm versions
  - Dynamic K-factor support (base K + new player bonus)
  - Default v1 (standard K=32) and v2 (dynamic K with bonus)

### 004: Seasons System
- **File**: `004_create_seasons_system.sql`
- **Purpose**: Sequential time periods with ELO resets
- **Tables**: `seasons`, `player_seasons`
- **Features**:
  - Sequential seasons (each ends when next begins)
  - Per-season player statistics
  - Season-specific ELO tracking
  - Historical "All-Time" season for pre-season data
  - Automatic game and ELO history association

### 005: Job Tracking System
- **File**: `005_create_job_tracking_system.sql`
- **Purpose**: Track long-running background operations
- **Tables**: `jobs`
- **Features**:
  - Job status tracking (pending, running, completed, failed)
  - Progress tracking with percentage and item counts
  - Result data storage (JSONB)

## Migration History

These migrations consolidate the original 9 scattered migrations into 5 logical groups:

**Original → Consolidated**
- 001, 003 → 001 (Authentication)
- 002, 008 → 002 (Table Tennis Core)
- 005, 007 → 003 (ELO Configuration)
- 009 → 004 (Seasons)
- 006 → 005 (Job Tracking)

## Database Reset

To reset and reapply all migrations:

```bash
sqlx database reset -y --database-url "$DATABASE_URL" --source backend/migrations
```

Or from the backend directory:

```bash
sqlx database reset -y
```

## Verification

After applying migrations, verify with:

```bash
# List all tables
psql "$DATABASE_URL" -c "\dt"

# Check migration history
psql "$DATABASE_URL" -c "SELECT version, description FROM _sqlx_migrations ORDER BY version;"
```

Expected tables:
- users, sessions (auth)
- players, games, elo_history (core)
- elo_configurations (ELO)
- seasons, player_seasons (seasons)
- jobs (background jobs)
- _sqlx_migrations (tracking)

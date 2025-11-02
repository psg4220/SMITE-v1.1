#!/bin/bash

# Load stored procedures into MySQL via docker

# Get environment variables from .env file
if [ -f ".env" ]; then
    export $(cat .env | grep -v '#' | xargs)
fi

# Get database credentials
DB_HOST="${DATABASE_URL##*@}"
DB_HOST="${DB_HOST%%/*}"
DB_HOST="${DB_HOST%%:*}"

# Extract credentials from DATABASE_URL
# Format: mysql://user:password@host:port/database
DB_CREDS="${DATABASE_URL##mysql://}"
DB_CREDS="${DB_CREDS%%@*}"
DB_USER="${DB_CREDS%%:*}"
DB_PASSWORD="${DB_CREDS#*:}"

DB_NAME="${DATABASE_URL##*/}"

# Try to find the mysql container
CONTAINER_NAME=$(docker ps --filter "label=com.docker.compose.service=db" --format "{{.Names}}" 2>/dev/null | head -1)

# If not found by label, try to find by image name
if [ -z "$CONTAINER_NAME" ]; then
    CONTAINER_NAME=$(docker ps --filter "ancestor=mysql" --format "{{.Names}}" 2>/dev/null | head -1)
fi

# If still not found, try common names
if [ -z "$CONTAINER_NAME" ]; then
    for name in mysql smite-db db database; do
        if docker ps --filter "name=$name" --format "{{.Names}}" 2>/dev/null | grep -q "$name"; then
            CONTAINER_NAME="$name"
            break
        fi
    done
fi

if [ -z "$CONTAINER_NAME" ]; then
    echo "❌ Could not find MySQL container"
    exit 1
fi

echo "Using container: $CONTAINER_NAME"

# Load procedures via docker exec
docker exec -i "$CONTAINER_NAME" mysql -u "$DB_USER" -p"$DB_PASSWORD" "$DB_NAME" < migrations/procedures.sql

# Check exit code
if [ $? -eq 0 ]; then
    echo "✓ Stored procedures loaded successfully"
    exit 0
else
    echo "❌ Failed to load stored procedures"
    exit 1
fi

#!/bin/sh
set -e

if [ ! -f /app/.env ]; then
  echo "⚠️  /app/.env non trovato, copio da .env.example"
  cp /app/.env.example /app/.env
fi

mkdir -p /app/output
mkdir -p /app/output/buffer

echo "🚀 Avvio nautilus-sonar..."
exec /app/nautilus-sonar
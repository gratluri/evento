#!/bin/bash
set -e

echo "Starting Podman/Docker Compose infrastructure..."
docker-compose up -d --build

echo "Waiting for infrastructure to be healthy (this takes some time for Cassandra and Kafka)..."
sleep 30

echo "Initializing Cassandra schema..."
# Docker compose exec cassandra cqlsh -f /init.cql
docker-compose exec cassandra cqlsh -e "CREATE KEYSPACE IF NOT EXISTS mykeyspace WITH replication = {'class':'SimpleStrategy', 'replication_factor':1}; USE mykeyspace; CREATE TABLE IF NOT EXISTS users (user_id int PRIMARY KEY, name text);"

echo "Starting Evento server on host (port 8080)..."
cargo run --bin evento -- server --port 8080 &
SERVER_PID=$!
sleep 3

echo "Submitting test plan to Evento..."
cargo run --bin evento-client -- run --plan tests/multi_protocol.yaml --server http://localhost:8080

echo "Waiting for test execution to complete..."
sleep 5

echo "Fetching recent test runs..."
cargo run --bin evento-client -- list --server http://localhost:8080

echo "Checking Postgres for inserted row..."
docker-compose exec postgres psql -U user -d evento_db -c "SELECT * FROM events;"

echo "Checking Cassandra for inserted row..."
docker-compose exec cassandra cqlsh -e "SELECT * FROM mykeyspace.users;"

echo "Shutting down Evento server..."
kill $SERVER_PID

echo "Done! Run 'docker-compose down' to cleanup."

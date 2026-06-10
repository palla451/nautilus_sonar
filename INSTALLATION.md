# Nautilus Sonar - Installation Guide

## Overview

This document describes how to install and start the current Nautilus Sonar environment.

The project is composed of two main parts:

```text
nautilus_sonar/
├── docker-compose.yml
├── src/
├── output/
├── opensearch/
├── dashboards/
└── nautilus-backend/
    └── backend/
```

Main components:

* Nautilus Sonar
* Nautilus Consumer
* Nautilus Detection Engine
* ValKey
* OpenSearch
* OpenSearch Dashboards
* Laravel Backend

---

# 1. Start Nautilus Sonar Stack

From the root directory of the project:

```bash
cd ~/Project/nautilus_sonar
```

Build the Docker images:

```bash
docker compose build
```

Start the stack:

```bash
docker compose up -d
```

Verify containers:

```bash
docker ps
```

Expected containers include:

```text
nautilus-sonar
nautilus-consumer
nautilus-correlator
nautilus-valkey
nautilus-opensearch
nautilus-opensearch-dashboards
```

---

# 2. Start Laravel Backend

Move into the backend directory:

```bash
cd ~/Project/nautilus_sonar/nautilus-backend/backend
```

Build the backend containers:

```bash
docker compose build
```

Start the backend:

```bash
docker compose up -d
```

Verify backend containers:

```bash
docker ps
```

---

# 3. Run Laravel Migrations

Enter the Laravel backend container:

```bash
docker exec -it nautilus-backend bash
```

Run database migrations:

```bash
php artisan migrate
```

Exit the container:

```bash
exit
```

Alternatively, run migrations directly:

```bash
docker exec -it nautilus-backend php artisan migrate
```

---

# 4. Clear Laravel Cache

After code or Blade changes, clear Laravel cache:

```bash
docker exec -it nautilus-backend php artisan optimize:clear
```

---

# 5. Access URLs

## Laravel Backend

Rule Manager UI:

```text
http://localhost:8080/rules
```

Create Rule page:

```text
http://localhost:8080/rules/create
```

Incidents page:

```text
http://localhost:8080/incidents
```

---

## OpenSearch

OpenSearch API:

```text
http://localhost:9200
```

Check cluster health:

```bash
curl http://localhost:9200/_cluster/health?pretty
```

Check Nautilus indexes:

```bash
curl http://localhost:9200/_cat/indices?v
```

---

## OpenSearch Dashboards

Dashboard UI:

```text
http://localhost:5601
```

---

# 6. Verify Rule Synchronization

After creating rules from Laravel, verify that the probe receives them.

From the Nautilus Sonar root:

```bash
cd ~/Project/nautilus_sonar
```

Check the probe ID:

```bash
cat output/probe_id
```

Query Laravel rules API:

```bash
PROBE_ID=$(cat output/probe_id)

curl -s "http://localhost:8080/api/probes/$PROBE_ID/rules" | jq
```

Expected result:

```text
suricata rules
aggregation rules
correlation rules
```

---

# 7. Verify Suricata Rules

Check generated Nautilus Suricata rules:

```bash
cat /var/lib/suricata/rules/nautilus.rules
```

Expected test rule:

```text
alert udp any any -> any 53 (msg:"NAUTILUS TEST - ANY DNS UDP 53"; sid:1000003; rev:1;)
```

---

# 8. Generate Test Traffic

Generate DNS traffic:

```bash
for i in {1..5}; do
    dig @8.8.8.8 google.com > /dev/null
done
```

Verify Suricata alert:

```bash
sudo grep '1000003' /var/log/suricata/eve.json | tail -5
```

---

# 9. Verify Events in OpenSearch

```bash
curl -s "http://localhost:9200/nautilus-events/_search?pretty" \
-H "Content-Type: application/json" \
-d '{
  "size": 5,
  "query": {
    "term": {
      "payload.signature_id": 1000003
    }
  }
}'
```

---

# 10. Verify Incidents in OpenSearch

```bash
curl -s "http://localhost:9200/nautilus-incidents/_search?pretty" \
-H "Content-Type: application/json" \
-d '{
  "size": 10,
  "sort": [
    {
      "timestamp": "desc"
    }
  ]
}'
```

Expected incident fields:

```json
{
  "incident_id": "uuid",
  "correlation_key": "deduplication-key",
  "incident_type": "dns_alert_aggregation_incident",
  "severity": "medium"
}
```

---

# 11. Logs

## Nautilus Sonar

```bash
docker logs nautilus-sonar --tail 100
```

## Nautilus Consumer

```bash
docker logs nautilus-consumer --tail 100
```

## Nautilus Detection Engine

```bash
docker logs nautilus-correlator --tail 100
```

Expected Detection Engine logs:

```text
Scarico regole centralizzate da Laravel
Aggregation rule caricata da Laravel
Correlation rule caricata da Laravel
Incidente aggregation indicizzato
Incidente correlation indicizzato
```

---

# 12. Clean Restart

To restart from a clean environment:

## Stop Nautilus Sonar stack

```bash
cd ~/Project/nautilus_sonar

docker compose down -v
rm -rf output
mkdir -p output
```

## Stop Laravel backend

```bash
cd ~/Project/nautilus_sonar/nautilus-backend/backend

docker compose down
```

Then restart following the installation steps above.

---

# Current Access Summary

```text
Laravel Rule Manager:
http://localhost:8080/rules

Laravel Create Rule:
http://localhost:8080/rules/create

Laravel Incidents:
http://localhost:8080/incidents

OpenSearch API:
http://localhost:9200

OpenSearch Dashboards:
http://localhost:5601
```

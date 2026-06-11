# Nautilus Sonar - Installation Guide

# Prerequisites

Before starting Nautilus Sonar, ensure the following software is installed:

* Docker
* Docker Compose
* Suricata (installed on the host operating system)
* curl
* jq

IMPORTANT

Nautilus Sonar does **not** currently deploy or manage a Suricata instance inside Docker.

Suricata must be installed and running directly on the host operating system.

Without a running Suricata instance, Nautilus will not generate events or incidents.

---

# Verify Suricata Installation

Verify Suricata is installed:

```bash
which suricata
```

Expected:

```text
/usr/bin/suricata
```

Verify configuration file:

```bash
sudo find / -name suricata.yaml
```

Expected:

```text
/etc/suricata/suricata.yaml
```

Verify service status:

```bash
sudo systemctl status suricata
```

Verify rule path configuration:

```bash
grep -n "default-rule-path" /etc/suricata/suricata.yaml

grep -n "rule-files" -A20 /etc/suricata/suricata.yaml
```

Expected:

```yaml
default-rule-path: /var/lib/suricata/rules

rule-files:
  - suricata.rules
  - nautilus.rules
```

Verify EVE log file:

```bash
ls -la /var/log/suricata/eve.json
```

---

# Overview

This document describes how to install, configure and validate the Nautilus Sonar platform.

The current architecture is composed of:

```text
Host Machine
├── Suricata
├── eve.json
└── Nautilus Docker Environment

nautilus_sonar/
├── docker-compose.yml
├── src/
├── output/
├── opensearch/
├── dashboards/
├── INSTALLATION.md
└── nautilus-backend/
    └── backend/
```

Main Components:

* Nautilus Sonar
* Nautilus Consumer
* Nautilus Detection Engine
* ValKey
* OpenSearch
* OpenSearch Dashboards
* Laravel Backend
* Suricata (Host OS)

---

# Detection Flow

```text
Laravel Rule Manager
        ↓
Rules API
        ↓
Nautilus Sonar
        ↓
/var/lib/suricata/rules/nautilus.rules
        ↓
Suricata Rule Reload
        ↓
Suricata Detection
        ↓
eve.json
        ↓
Nautilus Consumer
        ↓
nautilus-events
        ↓
Aggregation Rules
        ↓
nautilus-incidents
        ↓
Correlation Rules
        ↓
Correlated Incidents
```

---

# 1. Start Nautilus Stack

Move to the project root:

```bash
cd ~/Project/nautilus_sonar
```

Build containers:

```bash
docker compose build
```

Start containers:

```bash
docker compose up -d
```

Verify:

```bash
docker ps
```

Expected containers:

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

Move to backend:

```bash
cd ~/Project/nautilus_sonar/nautilus-backend/backend
```

Build:

```bash
docker compose build
```

Start:

```bash
docker compose up -d
```

Verify:

```bash
docker ps
```

Expected container:

```text
nautilus-backend
```

---

# 3. Laravel Initial Setup

Enter container:

```bash
docker exec -it nautilus-backend bash
```

Run:

```bash
composer install

cp .env.example .env

php artisan key:generate

chmod -R 775 storage bootstrap/cache

chown -R www-data:www-data storage bootstrap/cache

php artisan migrate
```

Clear cache:

```bash
php artisan optimize:clear
```

Exit:

```bash
exit
```

---

# 4. Access URLs

## Laravel Rule Manager

```text
http://localhost:8080/rules
```

## Create Rule

```text
http://localhost:8080/rules/create
```

## Incidents

```text
http://localhost:8080/incidents
```

## OpenSearch

```text
http://localhost:9200
```

## OpenSearch Dashboards

```text
http://localhost:5601
```

---

# 5. Verify Rule Synchronization

Verify probe identifier:

```bash
cat output/probe_id
```

Query Laravel:

```bash
PROBE_ID=$(cat output/probe_id)

curl -s \
"http://localhost:8080/api/probes/$PROBE_ID/rules" \
| jq
```

Expected:

```text
suricata rules
aggregation rules
correlation rules
```

---

# 6. Verify Generated Suricata Rules

Nautilus Sonar generates:

```text
/var/lib/suricata/rules/nautilus.rules
```

Verify:

```bash
cat /var/lib/suricata/rules/nautilus.rules
```

Example:

```suricata
alert dns any any -> any any \
(msg:"NAUTILUS TEST SID 999999";
 dns.query;
 content:"openai.com";
 nocase;
 sid:999999;
 rev:1;)
```

---

# 7. Reload Suricata Rules (Mandatory)

IMPORTANT

Every time a Suricata rule is created, modified or deleted from the Nautilus Rule Manager, Suricata must reload its rules.

Failure to reload the rules will cause Suricata to continue using previously loaded signatures.

Validate configuration:

```bash
sudo suricata -T -c /etc/suricata/suricata.yaml
```

Reload rules:

```bash
sudo suricatasc -c reload-rules
```

If unavailable:

```bash
sudo systemctl restart suricata
```

Verify configuration:

```bash
grep -n "default-rule-path" /etc/suricata/suricata.yaml

grep -n "rule-files" -A20 /etc/suricata/suricata.yaml
```

Expected:

```yaml
default-rule-path: /var/lib/suricata/rules

rule-files:
  - suricata.rules
  - nautilus.rules
```

---

# 8. Generate Test Traffic

Example test rule:

```suricata
alert dns any any -> any any \
(msg:"NAUTILUS TEST SID 999999";
 dns.query;
 content:"openai.com";
 nocase;
 sid:999999;
 rev:1;)
```

Generate traffic:

```bash
dig openai.com
```

---

# 9. Verify Suricata Detection

Verify eve.json:

```bash
sudo grep "999999" /var/log/suricata/eve.json
```

Expected:

```json
{
  "signature_id": 999999,
  "signature": "NAUTILUS TEST SID 999999"
}
```

---

# 10. Verify Events in OpenSearch

```bash
curl -s \
"http://localhost:9200/nautilus-events/_search?pretty" \
-H "Content-Type: application/json" \
-d '{
  "query": {
    "term": {
      "payload.signature_id": 999999
    }
  }
}'
```

Expected:

```json
{
  "event_type": "alert",
  "payload": {
    "signature_id": 999999
  }
}
```

---

# 11. Verify Aggregation Incidents

```bash
curl -s \
"http://localhost:9200/nautilus-incidents/_search?pretty" \
-H "Content-Type: application/json" \
-d '{
  "query": {
    "term": {
      "incident_type": "nautilus_test_alert"
    }
  }
}'
```

Expected:

```json
{
  "incident_type": "nautilus_test_alert",
  "severity": "medium"
}
```

---

# 12. Verify Correlation Incidents

```bash
curl -s \
"http://localhost:9200/nautilus-incidents/_search?pretty" \
-H "Content-Type: application/json" \
-d '{
  "query": {
    "term": {
      "incident_type": "nautilus_test_correlation"
    }
  }
}'
```

Expected:

```json
{
  "incident_type": "nautilus_test_correlation",
  "severity": "high"
}
```

---

# 13. End-to-End Validation Checklist

Successful validation requires:

* Rule visible in Laravel
* Rule returned by Rules API
* Rule written to `/var/lib/suricata/rules/nautilus.rules`
* Suricata rules reloaded
* Alert visible in `eve.json`
* Event visible in `nautilus-events`
* Aggregation incident visible in `nautilus-incidents`
* Correlation incident visible in `nautilus-incidents`

---

# 14. Logs

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

Expected:

```text
Rules downloaded
Aggregation rule loaded
Correlation rule loaded
Aggregation incident indexed
Correlation incident indexed
```

---

# 15. Clean Restart

Stop Nautilus stack:

```bash
cd ~/Project/nautilus_sonar

docker compose down -v

rm -rf output

mkdir -p output
```

Stop Laravel backend:

```bash
cd ~/Project/nautilus_sonar/nautilus-backend/backend

docker compose down
```

Restart from step 1.

---

# Current Access Summary

```text
Laravel Rule Manager
http://localhost:8080/rules

Create Rule
http://localhost:8080/rules/create

Incidents
http://localhost:8080/incidents

OpenSearch API
http://localhost:9200

OpenSearch Dashboards
http://localhost:5601
```

# Nautilus Sonar - Installation Guide

## Overview

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
* Suricata (installed on the host operating system)

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

```bash
cd ~/Project/nautilus_sonar
docker compose build
docker compose up -d
```

Verify containers:

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

```bash
cd ~/Project/nautilus_sonar/nautilus-backend/backend
docker compose build
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

Enter the backend container:

```bash
docker exec -it nautilus-backend bash
```

Execute:

```bash
composer install

cp .env.example .env

php artisan key:generate

chmod -R 775 storage bootstrap/cache

chown -R www-data:www-data storage bootstrap/cache

php artisan migrate
```

Clear Laravel cache:

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

The Sonar service generates:

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

# 7. Reload Suricata Rules

IMPORTANT

Updating nautilus.rules does not automatically activate the rule.

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

Example rule:

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

Check eve.json:

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

---

# 13. Logs

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

# 14. Clean Restart

Stop Nautilus:

```bash
cd ~/Project/nautilus_sonar

docker compose down -v

rm -rf output

mkdir -p output
```

Stop Laravel:

```bash
cd ~/Project/nautilus_sonar/nautilus-backend/backend

docker compose down
```

Restart from step 1.

---

# Current Access Summary

```text
Rule Manager
http://localhost:8080/rules

Create Rule
http://localhost:8080/rules/create

Incidents
http://localhost:8080/incidents

OpenSearch
http://localhost:9200

OpenSearch Dashboards
http://localhost:5601
```

#!/bin/sh

set -e

echo "⏳ Attendo OpenSearch..."

until curl -s http://opensearch:9200/_cluster/health >/dev/null
do
    sleep 5
done

echo "✅ OpenSearch disponibile"

echo "🧱 Creo indice nautilus-events con mapping corretto..."

curl -s -X PUT "http://opensearch:9200/nautilus-events" \
  -H "Content-Type: application/json" \
  -d '{
    "mappings": {
      "dynamic": true,
      "properties": {
        "timestamp": { "type": "date" },
        "event_type": { "type": "keyword" },
        "src_ip": { "type": "ip" },
        "src_port": { "type": "integer" },
        "dest_ip": { "type": "ip" },
        "dest_port": { "type": "integer" },
        "proto": { "type": "keyword" },
        "app_proto": { "type": "keyword" },
        "in_iface": { "type": "keyword" },
        "flow_id": { "type": "long" },
        "probe": {
          "properties": {
            "probe_id": { "type": "keyword" },
            "sensor_name": { "type": "keyword" },
            "version": { "type": "keyword" },
            "host": {
              "properties": {
                "hostname": { "type": "keyword" },
                "ip": { "type": "ip" },
                "os": { "type": "keyword" }
              }
            }
          }
        },
        "payload": {
          "properties": {
            "kind": { "type": "keyword" },
            "query": { "type": "keyword" },
            "query_type": { "type": "keyword" },
            "hostname": { "type": "keyword" },
            "http_method": { "type": "keyword" },
            "protocol": { "type": "keyword" },
            "status": { "type": "integer" },
            "length": { "type": "long" },
            "severity": { "type": "integer" },
            "signature_id": { "type": "long" },
            "signature": { "type": "text" },
            "category": { "type": "keyword" }
          }
        }
      }
    }
  }' || true

echo ""
echo "🧱 Creo indice nautilus-incidents con mapping corretto..."

curl -s -X PUT "http://opensearch:9200/nautilus-incidents" \
  -H "Content-Type: application/json" \
  -d '{
    "mappings": {
      "dynamic": true,
      "properties": {
        "timestamp": { "type": "date" },
        "incident_id": { "type": "keyword" },
        "rule_id": { "type": "keyword" },
        "rule_name": { "type": "keyword" },
        "incident_type": { "type": "keyword" },
        "severity": { "type": "keyword" },
        "description": { "type": "text" },
        "source_index": { "type": "keyword" },
        "evidence": {
          "properties": {
            "count": { "type": "long" },
            "window_seconds": { "type": "long" },
            "group_by": { "type": "keyword" },
            "values": {
              "properties": {
                "src_ip": { "type": "keyword" },
                "payload": {
                  "properties": {
                    "query": { "type": "keyword" }
                  }
                }
              }
            }
          }
        }
      }
    }
  }' || true

echo ""
echo "✅ Indici Nautilus inizializzati"
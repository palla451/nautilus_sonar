#!/bin/sh

set -e

echo "⏳ Attendo OpenSearch..."

until curl -s http://opensearch:9200 >/dev/null
do
    sleep 5
done

echo "✅ OpenSearch disponibile"

echo "📦 Creo index template nautilus-events-template..."

curl -s -X PUT \
  "http://opensearch:9200/_index_template/nautilus-events-template" \
  -H "Content-Type: application/json" \
  -d @/templates/nautilus-events-template.json

echo ""
echo "✅ Template nautilus-events-template creato"
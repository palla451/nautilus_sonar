#!/bin/sh

set -e

echo "⏳ Attendo OpenSearch Dashboards..."

until curl -s http://opensearch-dashboards:5601/api/status >/dev/null
do
    sleep 5
done

echo "✅ OpenSearch Dashboards disponibile"

echo "📊 Creo Data View Nautilus Events..."

curl -s -X POST \
  "http://opensearch-dashboards:5601/api/saved_objects/index-pattern/nautilus-events-data-view?overwrite=true" \
  -H "osd-xsrf: true" \
  -H "Content-Type: application/json" \
  -d '{
    "attributes": {
      "title": "nautilus-events*",
      "timeFieldName": "timestamp"
    }
  }'

echo ""
echo "📥 Importo dashboard Nautilus..."

curl -s -X POST \
  "http://opensearch-dashboards:5601/api/saved_objects/_import?overwrite=true" \
  -H "osd-xsrf: true" \
  --form file=@/dashboards/nautilus-dashboard.ndjson

echo ""
echo "✅ Dashboard Nautilus importata"
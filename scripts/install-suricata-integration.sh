#!/usr/bin/env bash
set -euo pipefail

SURICATA_CONFIG="/etc/suricata/suricata.yaml"
#SURICATA_RULES_DIR="/etc/suricata/rules"
SURICATA_RULES_DIR="/var/lib/suricata/rules"
NAUTILUS_RULE_FILE="${SURICATA_RULES_DIR}/nautilus.rules"

echo "🔧 Nautilus Suricata integration installer"

if [ ! -f "$SURICATA_CONFIG" ]; then
  echo "❌ Suricata config not found: $SURICATA_CONFIG"
  exit 1
fi

sudo mkdir -p "$SURICATA_RULES_DIR"

if [ ! -f "$NAUTILUS_RULE_FILE" ]; then
  echo "📄 Creating $NAUTILUS_RULE_FILE"
  echo "# Nautilus managed rules" | sudo tee "$NAUTILUS_RULE_FILE" >/dev/null
fi

if grep -q "nautilus.rules" "$SURICATA_CONFIG"; then
  echo "✅ nautilus.rules already included in suricata.yaml"
else
  echo "➕ Adding nautilus.rules to suricata.yaml"

  sudo cp "$SURICATA_CONFIG" "${SURICATA_CONFIG}.bak.$(date +%Y%m%d%H%M%S)"

  sudo python3 - <<'PY'
from pathlib import Path

path = Path("/etc/suricata/suricata.yaml")
text = path.read_text()

old = "rule-files:\n  - suricata.rules"
new = "rule-files:\n  - suricata.rules\n  - nautilus.rules"

if "  - nautilus.rules" not in text:
    if old not in text:
        raise SystemExit("Unable to find expected rule-files block")
    text = text.replace(old, new, 1)

path.write_text(text)
PY
fi

echo "🧪 Testing Suricata configuration"
sudo suricata -T -c "$SURICATA_CONFIG" -v

echo "🔁 Restarting Suricata"
sudo systemctl restart suricata

echo "✅ Nautilus Suricata integration completed"
<!DOCTYPE html>
<html>
<head>
    <title>Create Rule</title>
</head>
<body>
<h1>Create Rule</h1>

@if ($errors->any())
    <div style="color: red;">
        <ul>
            @foreach ($errors->all() as $error)
                <li>{{ $error }}</li>
            @endforeach
        </ul>
    </div>
@endif

<form method="POST" action="/rules">
    @csrf

    <p>
        <label>Name</label><br>
        <input type="text" name="name" value="{{ old('name', 'DNS Port 53 Alert') }}" required>
    </p>

    <p>
        <label>Description</label><br>
        <textarea name="description">{{ old('description', 'Detect any UDP DNS request to port 53') }}</textarea>
    </p>

    <p>
        <label>Type</label><br>
        <select name="type" id="type" onchange="toggleRuleFields()">
            <option value="aggregation" {{ old('type') === 'aggregation' ? 'selected' : '' }}>aggregation</option>
            <option value="correlation" {{ old('type') === 'correlation' ? 'selected' : '' }}>correlation</option>
            <option value="suricata" {{ old('type', 'suricata') === 'suricata' ? 'selected' : '' }}>suricata</option>
        </select>
    </p>

    <p>
        <label>Version</label><br>
        <input type="number" name="version" value="{{ old('version', 1) }}">
    </p>

    <p>
        <label>
            <input type="checkbox" name="enabled" checked>
            Enabled
        </label>
    </p>

    <hr>

    <div id="aggregation-fields">
        <h2>Aggregation Rule</h2>

        <p>
            <label>Source Index</label><br>
            <input type="text" name="source_index" value="{{ old('source_index', 'nautilus-events') }}">
        </p>

        <p>
            <label>Target Index</label><br>
            <input type="text" name="target_index" value="{{ old('target_index', 'nautilus-incidents') }}">
        </p>

        <p>
            <label>Filter Event Type</label><br>
            <input type="text" name="filter_event_type" value="{{ old('filter_event_type', 'alert') }}">
        </p>

        <p>
            <label>Filter Signature ID</label><br>
            <input type="number" name="filter_signature_id" value="{{ old('filter_signature_id', 1000003) }}">
        </p>

        <p>
            <label>Group By</label><br>
            <input type="text" name="group_by" value="{{ old('group_by', 'payload.signature_id') }}" size="80">
        </p>

        <p>
            <label>Threshold Count</label><br>
            <input type="number" name="threshold_count" value="{{ old('threshold_count', 2) }}">
        </p>

        <p>
            <label>Window Seconds</label><br>
            <input type="number" name="window_seconds" value="{{ old('window_seconds', 300) }}">
        </p>

        <p>
            <label>Severity</label><br>
            <select name="severity">
                <option value="low" {{ old('severity') === 'low' ? 'selected' : '' }}>low</option>
                <option value="medium" {{ old('severity', 'medium') === 'medium' ? 'selected' : '' }}>medium</option>
                <option value="high" {{ old('severity') === 'high' ? 'selected' : '' }}>high</option>
                <option value="critical" {{ old('severity') === 'critical' ? 'selected' : '' }}>critical</option>
            </select>
        </p>

        <p>
            <label>Incident Type</label><br>
            <input type="text" name="incident_type" value="{{ old('incident_type', 'dns_alert_aggregation_incident') }}">
        </p>

        <p>
            <label>Run Every Seconds</label><br>
            <input type="number" name="run_every_seconds" value="{{ old('run_every_seconds', 10) }}">
        </p>
    </div>

    <div id="json-fields" style="display:none;">
        <h2>Raw JSON Rule</h2>
        <p>
            <label>Content JSON</label><br>
            <textarea name="content" rows="14" cols="90">{{ old('content', '{}') }}</textarea>
        </p>
    </div>

    <div id="suricata-fields" style="display:none;">
        <h2>Suricata Rule</h2>
        <p>
            <label>Rule Content</label><br>
            <textarea name="suricata_rule" rows="8" cols="120">{{ old('suricata_rule', 'alert udp any any -> any 53 (msg:"NAUTILUS TEST - ANY DNS UDP 53"; sid:1000003; rev:1;)') }}</textarea>
        </p>
    </div>

    <button type="submit">Save Rule</button>
</form>

<p>
    <a href="/rules">Back</a>
</p>

<script>
    function toggleRuleFields() {
        const type = document.getElementById('type').value;

        document.getElementById('aggregation-fields').style.display =
            type === 'aggregation' ? 'block' : 'none';

        document.getElementById('json-fields').style.display =
            type === 'correlation' ? 'block' : 'none';

        document.getElementById('suricata-fields').style.display =
            type === 'suricata' ? 'block' : 'none';
    }

    toggleRuleFields();
</script>
</body>
</html>

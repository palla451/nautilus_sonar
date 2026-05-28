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
        <input type="text" name="name" value="{{ old('name', 'Repeated DNS Query') }}" required>
    </p>

    <p>
        <label>Description</label><br>
        <textarea name="description">{{ old('description', 'Detect repeated DNS queries from the same source to the same domain') }}</textarea>
    </p>

    <p>
        <label>Type</label><br>
        <select name="type" id="type" onchange="toggleRuleFields()">
            <option value="aggregation" selected>aggregation</option>
            <option value="correlation">correlation</option>
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
            <input type="text" name="filter_event_type" value="{{ old('filter_event_type', 'dns') }}">
        </p>

        <p>
            <label>Group By</label><br>
            <input type="text" name="group_by" value="{{ old('group_by', 'src_ip,payload.query') }}" size="80">
            <br>
            <small>Comma separated fields. Example: src_ip,payload.query</small>
        </p>

        <p>
            <label>Threshold Count</label><br>
            <input type="number" name="threshold_count" value="{{ old('threshold_count', 3) }}">
        </p>

        <p>
            <label>Window Seconds</label><br>
            <input type="number" name="window_seconds" value="{{ old('window_seconds', 300) }}">
        </p>

        <p>
            <label>Severity</label><br>
            <select name="severity">
                <option value="low" selected>low</option>
                <option value="medium">medium</option>
                <option value="high">high</option>
                <option value="critical">critical</option>
            </select>
        </p>

        <p>
            <label>Incident Type</label><br>
            <input type="text" name="incident_type" value="{{ old('incident_type', 'dns_repeated_query') }}">
        </p>

        <p>
            <label>Run Every Seconds</label><br>
            <input type="number" name="run_every_seconds" value="{{ old('run_every_seconds', 60) }}">
        </p>
    </div>

    <div id="json-fields" style="display:none;">
        <h2>Raw JSON Rule</h2>

        <p>
            <label>Content JSON</label><br>
            <textarea name="content" rows="14" cols="90">{
    "conditions": {
        "event_type": "dns",
        "destination.ip": "8.8.8.8",
        "destination.port": 53
    },
    "severity": "medium"
}</textarea>
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
            type === 'aggregation' ? 'none' : 'block';
    }

    toggleRuleFields();
</script>
</body>
</html>

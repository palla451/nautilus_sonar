<!DOCTYPE html>
<html>
<head>
    <title>Create Rule</title>
</head>
<body>
    <h1>Create Rule</h1>

    <form method="POST" action="/rules">
        @csrf

        <p>
            <label>Name</label><br>
            <input type="text" name="name" required>
        </p>

        <p>
            <label>Description</label><br>
            <textarea name="description"></textarea>
        </p>

        <p>
            <label>Type</label><br>
            <input type="text" name="type" value="correlation" required>
        </p>

        <p>
            <label>Version</label><br>
            <input type="number" name="version" value="1">
        </p>

        <p>
            <label>
                <input type="checkbox" name="enabled" checked>
                Enabled
            </label>
        </p>

        <p>
            <label>Content JSON</label><br>
            <textarea name="content" rows="12" cols="80" required>{
    "conditions": {
        "event_type": "dns",
        "destination.ip": "8.8.8.8",
        "destination.port": 53
    },
    "severity": "medium"
}</textarea>
        </p>

        <button type="submit">Save Rule</button>
    </form>

    <p>
        <a href="/rules">Back</a>
    </p>
</body>
</html>

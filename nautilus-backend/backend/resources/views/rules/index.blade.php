<!DOCTYPE html>
<html>
<head>
    <title>Nautilus Rules</title>
</head>
<body>
    <h1>Nautilus Rule Manager</h1>

    <p>
        <a href="/rules/create">Create new rule</a>
    </p>

    <table border="1" cellpadding="8">
        <thead>
            <tr>
                <th>Name</th>
                <th>UUID</th>
                <th>Type</th>
                <th>Version</th>
                <th>Enabled</th>
                <th>Created</th>
                <th>Action</th>
            </tr>
        </thead>
        <tbody>
            @foreach($rules as $rule)
                <tr>
                    <td>{{ $rule->name }}</td>
                    <td>{{ $rule->uuid }}</td>
                    <td>{{ $rule->type }}</td>
                    <td>{{ $rule->version }}</td>
                    <td>{{ $rule->enabled ? 'Yes' : 'No' }}</td>
                    <td>{{ $rule->created_at }}</td>
                    <td>
                        <form method="POST" action="/rules/{{ $rule->uuid }}">
                            @csrf
                            @method('DELETE')
                            <button type="submit">Delete</button>
                        </form>
                    </td>
                </tr>
            @endforeach
        </tbody>
    </table>
</body>
</html>

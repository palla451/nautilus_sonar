<!DOCTYPE html>
<html>
<head>
    <title>Nautilus Incidents</title>
</head>
<body>

<h1>Nautilus Incident Dashboard</h1>

<p>
    <a href="/rules">Rules</a> |
    <a href="/incidents">Incidents</a>
</p>

<hr>

@if($incidents->isEmpty())
    <p>No incidents found.</p>
@else

    <table border="1" cellpadding="8" cellspacing="0">
        <thead>
        <tr>
            <th>Status</th>
            <th>Severity</th>
            <th>Title</th>
            <th>Incident Type</th>
            <th>Rule</th>
            <th>Probe</th>
            <th>Destination</th>
            <th>Count</th>
            <th>Last Seen</th>
        </tr>
        </thead>

        <tbody>
        @foreach($incidents as $incident)

            @php
                $source = $incident['source'] ?? [];
            @endphp

            <tr>
                <td>
                    {{ $source['status'] ?? '-' }}
                </td>

                <td>
                    {{ $source['severity'] ?? '-' }}
                </td>

                <td>
                    {{ $source['title'] ?? '-' }}
                </td>

                <td>
                    {{ $source['incident_type'] ?? '-' }}
                </td>

                <td>
                    {{ data_get($source, 'rule.name', '-') }}
                </td>

                <td>
                    {{ data_get($source, 'probe.id', '-') }}
                </td>

                <td>
                    {{ data_get($source, 'destination.ip', '-') }}
                    :
                    {{ data_get($source, 'destination.port', '-') }}
                </td>

                <td>
                    {{ $source['event_count'] ?? 0 }}
                </td>

                <td>
                    {{ $source['last_seen'] ?? '-' }}
                </td>
            </tr>

        @endforeach
        </tbody>
    </table>

@endif

</body>
</html>

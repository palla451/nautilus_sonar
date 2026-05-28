<?php

namespace App\Http\Controllers\Api;

use App\Http\Controllers\Controller;
use App\Services\OpenSearchService;
use Illuminate\Http\Request;
use Illuminate\Support\Str;

class IncidentIngestionController extends Controller
{
    public function ingest(Request $request, OpenSearchService $opensearch)
    {
        $payload = $request->all();

        $incident = [
            'uuid' => $payload['uuid'] ?? (string) Str::uuid(),
            'created_at' => $payload['created_at'] ?? now()->toISOString(),
            'last_seen' => $payload['last_seen'] ?? now()->toISOString(),
            'event_count' => $payload['event_count'] ?? 1,

            'severity' => $payload['severity'] ?? 'low',
            'status' => $payload['status'] ?? 'open',
            'title' => $payload['title']
                ?? 'Local detection: ' . ($payload['rule_name'] ?? $payload['incident_type'] ?? 'unknown'),

            'incident_type' => $payload['incident_type'] ?? null,
            'source' => $payload['source'] ?? 'probe',

            'rule' => [
                'uuid' => $payload['rule_uuid'] ?? null,
                'name' => $payload['rule_name'] ?? null,
            ],

            'probe' => [
                'id' => data_get($payload, 'last_event.probe.probe_id')
                    ?? data_get($payload, 'last_event.probe.id')
                        ?? $payload['probe_id']
                        ?? null,
            ],

            'destination' => [
                'ip' => data_get($payload, 'last_event.dest_ip')
                    ?? data_get($payload, 'last_event.destination.ip'),
                'port' => data_get($payload, 'last_event.dest_port')
                    ?? data_get($payload, 'last_event.destination.port'),
            ],

            'group_key' => $payload['group_key'] ?? null,
            'window_seconds' => $payload['window_seconds'] ?? null,
            'last_event' => $payload['last_event'] ?? null,
            'raw_incident' => $payload,
        ];

        $opensearch->index(
            $incident,
            env('OPENSEARCH_INCIDENT_INDEX', 'nautilus-incidents')
        );

        return response()->json([
            'success' => true,
            'message' => 'Incident ingested',
            'data' => $incident,
        ], 201);
    }
}

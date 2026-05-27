<?php

namespace App\Http\Controllers\Api;

use App\Http\Controllers\Controller;
use App\Models\Probe;
use App\Models\Rule;
use App\Services\OpenSearchService;
use Illuminate\Http\Request;
use Illuminate\Support\Str;

class EventIngestionController extends Controller
{
    public function ingest(Request $request, OpenSearchService $opensearch)
    {
        $payload = $request->all();

        $probeId =
            data_get($payload, 'probe.id')
            ?? data_get($payload, 'probe.probe_id')
            ?? ($payload['probe_id'] ?? null);

        $normalized = [
            'timestamp' => $payload['timestamp'] ?? now()->toISOString(),
            'received_at' => now()->toISOString(),

            'probe' => [
                'id' => $probeId,
                'sensor_name' => data_get($payload, 'probe.sensor_name'),
                'version' => data_get($payload, 'probe.version'),
                'host' => data_get($payload, 'probe.host'),
            ],

            'in_iface' => $payload['in_iface'] ?? null,
            'flow_id' => $payload['flow_id'] ?? null,
            'event_type' => $payload['event_type'] ?? null,

            'source' => [
                'ip' => $payload['src_ip'] ?? data_get($payload, 'source.ip'),
                'port' => $payload['src_port'] ?? data_get($payload, 'source.port'),
            ],

            'destination' => [
                'ip' => $payload['dest_ip'] ?? data_get($payload, 'destination.ip'),
                'port' => $payload['dest_port'] ?? data_get($payload, 'destination.port'),
            ],

            'network' => [
                'transport' => $payload['proto'] ?? data_get($payload, 'network.transport'),
                'application' => $payload['app_proto'] ?? data_get($payload, 'network.application'),
            ],

            'payload' => $payload['payload'] ?? [],
            'raw_event' => $payload,
        ];

        if ($probeId) {
            Probe::updateOrCreate(
                ['uuid' => $probeId],
                [
                    'name' => data_get($payload, 'probe.sensor_name') ?? 'unknown-probe',
                    'hostname' => data_get($payload, 'probe.host.hostname'),
                    'ip_address' => data_get($payload, 'probe.host.ip') ?? $request->ip(),
                    'version' => data_get($payload, 'probe.version'),
                    'status' => 'online',
                    'last_heartbeat_at' => now(),
                    'metadata' => [
                        'in_iface' => $payload['in_iface'] ?? null,
                        'os' => data_get($payload, 'probe.host.os'),
                        'auto_registered' => true,
                    ],
                ]
            );
        }

        $opensearch->index($normalized);

        $matchedRules = $this->matchRules($normalized);

        foreach ($matchedRules as $rule) {
            $incidentIndex = env('OPENSEARCH_INCIDENT_INDEX', 'nautilus-incidents');

            $existingIncident = $this->findExistingIncident(
                $opensearch,
                $incidentIndex,
                $rule,
                $normalized
            );

            if ($existingIncident) {
                $source = $existingIncident['_source'];

                $opensearch->update(
                    $existingIncident['_id'],
                    [
                        'last_seen' => now()->toISOString(),
                        'event_count' => ($source['event_count'] ?? 1) + 1,
                        'last_event' => $normalized,
                    ],
                    $incidentIndex
                );

                continue;
            }

            $incident = [
                'uuid' => (string) Str::uuid(),
                'created_at' => now()->toISOString(),
                'last_seen' => now()->toISOString(),
                'event_count' => 1,
                'severity' => $rule['severity'] ?? 'medium',
                'status' => 'open',
                'title' => 'Rule matched: ' . $rule['name'],

                'rule' => [
                    'uuid' => $rule['uuid'],
                    'name' => $rule['name'],
                    'type' => $rule['type'],
                    'version' => $rule['version'],
                ],

                'probe' => [
                    'id' => data_get($normalized, 'probe.id'),
                ],

                'destination' => [
                    'ip' => data_get($normalized, 'destination.ip'),
                    'port' => data_get($normalized, 'destination.port'),
                ],

                'event' => $normalized,
                'last_event' => $normalized,
            ];

            $opensearch->index($incident, $incidentIndex);
        }

        return response()->json([
            'success' => true,
            'message' => 'Event ingested',
            'matched_rules' => count($matchedRules),
            'data' => $normalized,
        ]);
    }

    private function matchRules(array $event): array
    {
        $matched = [];

        $rules = Rule::where('enabled', true)
            ->where('type', 'correlation')
            ->get();

        foreach ($rules as $rule) {
            $content = json_decode($rule->content, true);

            if (!is_array($content)) {
                continue;
            }

            $conditions = $content['conditions'] ?? [];

            if ($this->matchConditions($event, $conditions)) {
                $matched[] = [
                    'uuid' => $rule->uuid,
                    'name' => $rule->name,
                    'type' => $rule->type,
                    'version' => $rule->version,
                    'severity' => $content['severity'] ?? 'medium',
                ];
            }
        }

        return $matched;
    }

    private function matchConditions(array $event, array $conditions): bool
    {
        foreach ($conditions as $field => $expectedValue) {
            $actualValue = data_get($event, $field);

            if ((string) $actualValue !== (string) $expectedValue) {
                return false;
            }
        }

        return true;
    }

    private function findExistingIncident(
        OpenSearchService $opensearch,
        string $index,
        array $rule,
        array $event
    ): ?array {
        $result = $opensearch->search([
            'size' => 1,

            'query' => [
                'bool' => [
                    'must' => [
                        [
                            'match_phrase' => [
                                'rule.uuid' => $rule['uuid'],
                            ],
                        ],
                        [
                            'match_phrase' => [
                                'probe.id' => data_get($event, 'probe.id'),
                            ],
                        ],
                        [
                            'match_phrase' => [
                                'destination.ip' => data_get($event, 'destination.ip'),
                            ],
                        ],
                        [
                            'term' => [
                                'destination.port' => data_get($event, 'destination.port'),
                            ],
                        ],
                        [
                            'match_phrase' => [
                                'status' => 'open',
                            ],
                        ],
                    ],
                ],
            ],

            'sort' => [
                [
                    'last_seen' => [
                        'order' => 'desc',
                    ],
                ],
            ],
        ], $index);

        return $result['hits']['hits'][0] ?? null;
    }
}

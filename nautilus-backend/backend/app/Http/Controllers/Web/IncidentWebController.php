<?php

namespace App\Http\Controllers\Web;

use App\Http\Controllers\Controller;
use App\Services\OpenSearchService;
use OpenSearch\Common\Exceptions\Missing404Exception;
use Throwable;

class IncidentWebController extends Controller
{
    public function index(OpenSearchService $opensearch)
    {
        try {
            $result = $opensearch->search([
                'size' => 50,
                'sort' => [
                    [
                        'last_seen' => [
                            'order' => 'desc',
                            'unmapped_type' => 'date',
                        ],
                    ],
                ],
            ], env('OPENSEARCH_INCIDENT_INDEX', 'nautilus-incidents'));

            $incidents = collect($result['hits']['hits'] ?? [])
                ->map(function ($hit) {
                    return [
                        'id' => $hit['_id'],
                        'source' => $hit['_source'],
                    ];
                });

        } catch (Missing404Exception $e) {
            $incidents = collect();

        } catch (Throwable $e) {
            $incidents = collect();
        }

        return view('incidents.index', compact('incidents'));
    }
}

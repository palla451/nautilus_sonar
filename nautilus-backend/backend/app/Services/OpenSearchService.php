<?php

namespace App\Services;

use OpenSearch\ClientBuilder;

class OpenSearchService
{
    protected $client;

    public function __construct()
    {
        $this->client = ClientBuilder::create()
            ->setHosts([
                env('OPENSEARCH_HOST')
            ])
            ->build();
    }

    public function index(array $data, ?string $index = null)
    {
        return $this->client->index([
            'index' => $index ?? env('OPENSEARCH_INDEX', 'nautilus-events'),
            'body' => $data
        ]);
    }

    public function search(array $query, ?string $index = null)
    {
        return $this->client->search([
            'index' => $index ?? env('OPENSEARCH_INDEX', 'nautilus-events'),
            'body' => $query
        ]);
    }

    public function update(string $id, array $data, ?string $index = null)
    {
        return $this->client->update([
            'index' => $index ?? env('OPENSEARCH_INDEX', 'nautilus-events'),
            'id' => $id,
            'body' => [
                'doc' => $data
            ]
        ]);
    }
}

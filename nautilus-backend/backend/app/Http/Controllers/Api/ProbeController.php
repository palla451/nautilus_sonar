<?php

namespace App\Http\Controllers\Api;

use App\Http\Controllers\Controller;
use App\Models\Probe;
use Illuminate\Http\Request;
use Illuminate\Support\Str;

class ProbeController extends Controller
{
    public function index()
    {
        return response()->json([
            'data' => Probe::orderByDesc('last_heartbeat_at')->get(),
        ]);
    }

    public function register(Request $request)
    {
        $data = $request->validate([
            'uuid' => ['nullable', 'uuid'],
            'name' => ['nullable', 'string'],
            'hostname' => ['nullable', 'string'],
            'ip_address' => ['nullable', 'string'],
            'version' => ['nullable', 'string'],
            'metadata' => ['nullable', 'array'],
        ]);

        $uuid = $data['uuid'] ?? (string) Str::uuid();

        $probe = Probe::updateOrCreate(
            ['uuid' => $uuid],
            [
                'name' => $data['name'] ?? null,
                'hostname' => $data['hostname'] ?? null,
                'ip_address' => $data['ip_address'] ?? $request->ip(),
                'version' => $data['version'] ?? null,
                'status' => 'online',
                'last_heartbeat_at' => now(),
                'metadata' => $data['metadata'] ?? null,
            ]
        );

        return response()->json([
            'message' => 'Probe registered successfully',
            'data' => $probe,
        ], 201);
    }

    public function heartbeat(Request $request)
    {
        $data = $request->validate([
            'uuid' => ['required', 'uuid'],
            'status' => ['nullable', 'string'],
            'metadata' => ['nullable', 'array'],
        ]);

        $probe = Probe::where('uuid', $data['uuid'])->firstOrFail();

        $probe->update([
            'status' => $data['status'] ?? 'online',
            'last_heartbeat_at' => now(),
            'metadata' => $data['metadata'] ?? $probe->metadata,
        ]);

        return response()->json([
            'message' => 'Heartbeat received',
            'data' => $probe,
        ]);
    }
}

<?php

namespace App\Http\Controllers\Api;

use App\Http\Controllers\Controller;
use App\Models\Rule;
use Illuminate\Http\Request;
use Illuminate\Support\Str;
use Throwable;

class RuleController extends Controller
{
    public function index()
    {
        try {
            return response()->json([
                'data' => Rule::orderByDesc('created_at')->get(),
            ]);
        } catch (Throwable $e) {
            return response()->json([
                'message' => 'Rules listing failed',
                'error' => $e->getMessage(),
            ], 500);
        }
    }

    public function store(Request $request)
    {
        try {
            $payload = $request->json()->all();

            $name = $payload['name'] ?? null;
            $type = $payload['type'] ?? 'correlation';
            $content = $payload['content'] ?? null;

            if (!$name) {
                return response()->json(['message' => 'The name field is required.'], 422);
            }

            if ($content === null) {
                return response()->json(['message' => 'The content field is required.'], 422);
            }

            if (is_array($content) || is_object($content)) {
                $content = json_encode($content, JSON_PRETTY_PRINT | JSON_UNESCAPED_SLASHES);
            }


            $rule = Rule::create([
                'uuid' => $payload['uuid'] ?? (string) Str::uuid(),
                'name' => $name,
                'description' => $payload['description'] ?? null,
                'type' => $type,
                'content' => $content,
                'version' => $payload['version'] ?? 1,
                'enabled' => $payload['enabled'] ?? true,
            ]);

            return response()->json([
                'message' => 'Rule created successfully',
                'data' => $rule,
            ], 201);

        } catch (Throwable $e) {
            return response()->json([
                'message' => 'Rule creation failed',
                'error' => $e->getMessage(),
            ], 500);
        }
    }

    public function active()
    {
        return response()->json([
            'data' => Rule::where('enabled', true)
                ->orderBy('type')
                ->orderBy('name')
                ->get(),
        ]);
    }

    public function destroy($uuid)
    {
        $rule = Rule::where('uuid', $uuid)->first();

        if (!$rule) {
            return response()->json([
                'message' => 'Rule not found',
            ], 404);
        }

        $rule->delete();

        return response()->json([
            'message' => 'Rule deleted successfully',
        ]);
    }

    public function rulesForProbe($uuid)
    {
        $rules = Rule::where('enabled', true)
            ->orderBy('type')
            ->orderBy('name')
            ->get()
            ->map(function ($rule) {
                return [
                    'uuid' => $rule->uuid,
                    'name' => $rule->name,
                    'description' => $rule->description,
                    'type' => $rule->type,
                    'version' => $rule->version,
                    'content' => json_decode($rule->content, true) ?? $rule->content,
                ];
            });

        return response()->json([
            'probe_uuid' => $uuid,
            'ruleset_version' => now()->timestamp,
            'rules_count' => $rules->count(),
            'rules' => $rules,
        ]);
    }
}

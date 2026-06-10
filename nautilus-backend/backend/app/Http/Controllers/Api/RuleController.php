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
        $validated = $request->validate([
            'name' => 'required|string|max:255',
            'description' => 'nullable|string',
            'type' => 'required|string|in:aggregation,correlation,suricata',
            'version' => 'nullable|integer',
            'enabled' => 'nullable',
        ]);

        $type = $request->input('type');

        if ($type === 'aggregation') {
            $content = [
                'source_index' => $request->input('source_index'),
                'target_index' => $request->input('target_index'),
                'filter_event_type' => $request->input('filter_event_type'),
                'group_by' => array_map('trim', explode(',', $request->input('group_by'))),
                'threshold_count' => (int) $request->input('threshold_count'),
                'window_seconds' => (int) $request->input('window_seconds'),
                'severity' => $request->input('severity'),
                'incident_type' => $request->input('incident_type'),
                'run_every_seconds' => (int) $request->input('run_every_seconds'),
            ];
        } elseif ($type === 'suricata') {
            $request->validate([
                'suricata_rule' => 'required|string',
            ]);

            $content = [
                'engine' => 'suricata',
                'rule' => $request->input('suricata_rule'),
            ];
        } else {
            $request->validate([
                'content' => 'required|json',
            ]);

            $content = json_decode($request->input('content'), true);
        }

        Rule::create([
            'name' => $request->input('name'),
            'description' => $request->input('description'),
            'type' => $type,
            'version' => $request->input('version', 1),
            'enabled' => $request->has('enabled'),
            'content' => $content,
        ]);

        return redirect('/rules')->with('success', 'Rule created successfully');
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

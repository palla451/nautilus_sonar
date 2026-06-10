<?php

namespace App\Http\Controllers\Web;

use App\Http\Controllers\Controller;
use App\Models\Rule;
use Illuminate\Http\Request;
use Illuminate\Support\Str;

class RuleWebController extends Controller
{
    public function index()
    {
        $rules = Rule::orderByDesc('created_at')->get();

        return view('rules.index', compact('rules'));
    }

    public function create()
    {
        return view('rules.create');
    }

    public function store(Request $request)
    {
        $request->validate([
            'name' => ['required', 'string'],
            'description' => ['nullable', 'string'],
            'type' => ['required', 'string', 'in:aggregation,correlation,suricata'],
            'version' => ['nullable', 'integer'],
            'enabled' => ['nullable'],
        ]);

        $type = $request->input('type');

        if ($type === 'aggregation') {
            $filter = [
                'event_type' => $request->input('filter_event_type', 'alert'),
            ];

            if ($request->filled('filter_signature_id')) {
                $filter['payload.signature_id'] = (int) $request->input('filter_signature_id');
            }

            $content = [
                'id' => Str::slug($request->input('name'), '_'),
                'source_index' => $request->input('source_index', 'nautilus-events'),
                'target_index' => $request->input('target_index', 'nautilus-incidents'),
                'filter' => $filter,
                'group_by' => array_values(array_filter(array_map(
                    'trim',
                    explode(',', $request->input('group_by', ''))
                ))),
                'threshold' => [
                    'count' => (int) $request->input('threshold_count', 3),
                    'window_seconds' => (int) $request->input('window_seconds', 300),
                ],
                'severity' => $request->input('severity', 'low'),
                'incident_type' => $request->input(
                    'incident_type',
                    Str::slug($request->input('name'), '_')
                ),
                'run_every_seconds' => (int) $request->input('run_every_seconds', 60),
            ];
        } elseif ($type === 'suricata') {
            $request->validate([
                'suricata_rule' => ['required', 'string'],
            ]);

            $content = [
                'engine' => 'suricata',
                'rule' => trim($request->input('suricata_rule')),
            ];
        } else {
            $content = json_decode($request->input('content'), true);

            if (!is_array($content)) {
                return back()
                    ->withInput()
                    ->withErrors([
                        'content' => 'Invalid JSON content.',
                    ]);
            }
        }

        Rule::create([
            'uuid' => (string) Str::uuid(),
            'name' => $request->input('name'),
            'description' => $request->input('description'),
            'type' => $type,
            'content' => json_encode($content, JSON_PRETTY_PRINT | JSON_UNESCAPED_SLASHES),
            'version' => $request->input('version', 1),
            'enabled' => $request->has('enabled'),
        ]);

        return redirect('/rules');
    }

    public function destroy($uuid)
    {
        Rule::where('uuid', $uuid)->delete();

        return redirect('/rules');
    }
}

<?php

namespace App\Http\Controllers\Api;

use App\Http\Controllers\Controller;
use App\Models\Rule;
use Illuminate\Http\Response;

class SuricataRuleController extends Controller
{
    public function index(): Response
    {
        $rules = Rule::query()
            ->where('type', 'suricata')
            ->where('enabled', true)
            ->orderBy('id')
            ->get();

        $content = $rules
            ->map(function (Rule $rule) {
                return $rule->content['rule'] ?? null;
            })
            ->filter()
            ->implode("\n\n");

        return response($content . "\n", 200)
            ->header('Content-Type', 'text/plain');
    }
}

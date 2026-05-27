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
            'type' => ['required', 'string'],
            'content' => ['required', 'string'],
            'version' => ['nullable', 'integer'],
            'enabled' => ['nullable'],
        ]);

        Rule::create([
            'uuid' => (string) Str::uuid(),
            'name' => $request->name,
            'description' => $request->description,
            'type' => $request->type,
            'content' => $request->content,
            'version' => $request->version ?? 1,
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

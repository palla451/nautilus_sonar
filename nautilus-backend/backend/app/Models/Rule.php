<?php

namespace App\Models;

use Illuminate\Database\Eloquent\Model;

class Rule extends Model
{
    protected $fillable = [
        'uuid',
        'name',
        'description',
        'type',
        'content',
        'version',
        'enabled',
    ];

    protected $casts = [
        'enabled' => 'boolean',
        'content' => 'array',
    ];
}

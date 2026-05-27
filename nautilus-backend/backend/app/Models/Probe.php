<?php

namespace App\Models;

use Illuminate\Database\Eloquent\Model;

class Probe extends Model
{
    protected $fillable = [
        'uuid',
        'name',
        'hostname',
        'ip_address',
        'version',
        'status',
        'last_heartbeat_at',
        'metadata',
    ];

    protected $casts = [
        'metadata' => 'array',
        'last_heartbeat_at' => 'datetime',
    ];
}

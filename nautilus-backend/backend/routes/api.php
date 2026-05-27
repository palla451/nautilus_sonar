<?php

use Illuminate\Support\Facades\Route;
use App\Http\Controllers\Api\ProbeController;
use App\Http\Controllers\Api\EventIngestionController;
use App\Http\Controllers\Api\RuleController;


Route::get('/probes', [ProbeController::class, 'index']);
Route::post('/probes/register', [ProbeController::class, 'register']);
Route::post('/probes/heartbeat', [ProbeController::class, 'heartbeat']);

Route::post(
    '/events/ingest',
    [EventIngestionController::class,'ingest']
);


Route::get('/rules', [RuleController::class, 'index']);
Route::post('/rules', [RuleController::class, 'store']);
Route::get('/rules/active', [RuleController::class, 'active']);
Route::delete('/rules/{uuid}', [RuleController::class, 'destroy']);

Route::get('/probes/{uuid}/rules', [RuleController::class, 'rulesForProbe']);

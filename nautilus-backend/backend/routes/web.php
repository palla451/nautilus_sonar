<?php

use Illuminate\Support\Facades\Route;
use App\Http\Controllers\Web\RuleWebController;

Route::get('/', function () {
    return redirect('/rules');
});

Route::get('/rules', [RuleWebController::class, 'index']);
Route::get('/rules/create', [RuleWebController::class, 'create']);
Route::post('/rules', [RuleWebController::class, 'store']);
Route::delete('/rules/{uuid}', [RuleWebController::class, 'destroy']);

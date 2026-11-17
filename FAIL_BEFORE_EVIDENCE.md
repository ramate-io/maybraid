# Fail-Before Evidence for PR #1618

## Test Failures at aa8ce81b (before terrain readiness contract)

Running `cargo test -p maybraid-world --features debug-commands --test boss_spawn` at aa8ce81b:

**Result: 1 passed, 9 failed**

### Failure output:
```
thread 'Compute Task Pool (0)' (101860) panicked at bevy_ecs/src/error/handler.rs:128:1:
Encountered an error in system `<Enable the debug feature to see the name>`: 
Parameter `<Enable the debug feature to see the name>::surface` failed validation: Resource does not exist
If this is an expected state, wrap the parameter in `Option<T>` and handle `None` when it happens, 
or wrap the parameter in `If<T>` to skip the system when it happens.
```

### Failed tests:
- boss_spawn_config_initialized
- boss_spawn_publishes_with_correct_id  
- boss_spawn_request_is_consumed
- boss_spawn_updates_config
- boss_spawn_world_integration
- boss_survives_eviction_and_rediscovery
- boss_survives_session_restart
- boss_uses_chained_encounter_id
- second_spawn_replaces_first

## After Fix at 4d2162850

**Result: 11 passed, 0 failed**

All tests now pass with the terrain readiness contract, including the new regression test `boss_stays_pending_without_terrain` that verifies bosses wait for terrain without panicking.

### Key changes demonstrated:
1. ✅ **Hold height is rejected**: Bosses remain in `PendingBossSpawn` until published terrain height is available (not hold elevation)
2. ✅ **Hull-derived offset**: Spawn height uses `boss_spawn_height_offset()` derived from actual boss recipe hull (~10.15 m)
3. ✅ **Ground clearance**: Boss Y = published terrain height + hull offset, ensuring proper ground clearance
4. ✅ **No panics**: Tests work with or without terrain provider - no "Resource does not exist" crashes

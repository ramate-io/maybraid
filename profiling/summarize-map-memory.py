"""Offline, bounded map-arm process-memory summary. No capture, export or owner scan."""
import argparse
import csv
from datetime import datetime, timezone
import hashlib
import json
import math
from pathlib import Path
import re
from statistics import median

PHASES = ('settled', 'hitch-walk', 'living-map', 'respawn')
MAX_GAP_SECONDS = 7.5  # Five-second cadence; a missing ordinary sample produces a 10s gap.
REVISIT_HALF_WINDOW_SECONDS = 10


def require(ok, message):
    if not ok:
        raise ValueError(message)


def utc(value):
    stamp = datetime.fromisoformat(value.replace('Z', '+00:00'))
    require(stamp.tzinfo is not None, 'UTC timestamp has no zone')
    return stamp.timestamp()


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def records(path):
    return [json.loads(line) for line in path.read_text().splitlines() if line.strip()]


def summarize(run, scope='whole-run', binding_sha256=None, termination_receipt=None):
    require(scope in ('whole-run', 'settled-walk'), 'unknown analysis scope')
    partial = scope == 'settled-walk'
    selected_phases = PHASES[:2] if partial else PHASES
    binding = json.loads((run/'binding.json').read_text())
    if partial:
        require(binding_sha256 is not None and re.fullmatch('[0-9a-f]{64}', binding_sha256)
                and sha(run/'binding.json') == binding_sha256, 'admitted binding digest mismatch')
        require(all(re.fullmatch('[0-9a-f]{64}', binding.get(k, ''))
                    for k in ('binary_sha256', 'manifest_sha256')), 'missing source/artifact binding')
        require(termination_receipt is not None and
                Path(binding['deadline_receipt']).resolve() == Path(termination_receipt).resolve(),
                'termination receipt is not capture-bound')
        termination_receipt = Path(termination_receipt)
        termination = json.loads(termination_receipt.read_text())
        require(termination.get('owned_group_remaining') is False, 'termination cleanup unverified')
        distance = json.loads((run/'distance.json').read_text())
        require(distance.get('goal_reached') is True and distance.get('active_xz_m', 0) >= 2000
                and distance.get('minimum_health') == 100 and len(distance.get('lives', [])) == 1,
                'distance/healthy actor goal not admitted')
    else:
        completion = json.loads((run/'completed.json').read_text())
        require(completion.get('normal_exit') is True and completion.get('distance_goal_reached') is True,
                'arm is not a normally completed distance-admitted capture')
        require(completion.get('binding_sha256') == sha(run/'binding.json'), 'completion/binding mismatch')
    bounds = {}
    for line in (run/'game.log').read_text(errors='replace').splitlines():
        match = re.search(r'agent\.scenario (\S+) (Start|End)\s*$', line)
        if not match or match[1] not in selected_phases:
            continue
        name, endpoint = match.groups()
        require(endpoint not in bounds.setdefault(name, {}), 'duplicate UTC phase marker')
        bounds[name][endpoint] = utc(line.split()[0])
    require(all(set(bounds.get(n, {})) == {'Start', 'End'} for n in selected_phases),
            'missing UTC phase marker')
    events = records(run/'events.jsonl')
    life = records(run/'lifecycle.jsonl')
    launched = [e for e in life if e.get('kind') == 'game_launched']
    require(len(launched) == 1, 'missing unique game launch')
    pid = launched[0]['pid']
    require(json.loads((run/'collector-game-pid.json').read_text())['pid'] == pid, 'game PID receipts disagree')
    if partial:
        begin, end = bounds['settled']['Start'], bounds['hitch-walk']['End']
        require(utc(termination['start_utc']) <= utc(launched[0]['utc']) <= begin < end
                <= utc(termination['end_utc']), 'phases outside capture termination bounds')
    else:
        exited = [e for e in life if e.get('kind') == 'game_exited']
        require(len(exited) == 1 and exited[0].get('exit_status') == 0,
                'missing unique successful game lifetime')
        begin, end = utc(launched[0]['utc']), utc(exited[0]['utc'])
        require(begin < end, 'invalid game lifetime')
    with (run/'memory.csv').open() as stream:
        raw = list(csv.DictReader(stream))
    require(raw, 'no memory samples')
    samples = []
    for r in raw:
        require(int(r['pid']) == pid, 'sampler PID mismatch')
        start = int(r['process_start_abstime'])
        require(start > 0, 'missing process-start identity')
        elapsed = float(r['elapsed_s'])
        require(math.isfinite(elapsed) and elapsed >= 0, 'invalid sampler elapsed time')
        row = dict(utc=r['utc'], time=utc(r['utc']), elapsed_s=elapsed, process_start_abstime=start,
                   rss_bytes=int(r['rss_bytes']), phys_footprint_bytes=int(r['phys_footprint_bytes']))
        require(row['rss_bytes'] >= 0 and row['phys_footprint_bytes'] >= 0, 'negative memory counter')
        samples.append(row)
    require(len({r['process_start_abstime'] for r in samples}) == 1, 'process-start identity changed')
    coverage = [r for r in samples if begin <= r['time'] <= end] if partial else samples
    for a, b in zip(coverage, coverage[1:]):
        wall, elapsed = b['time']-a['time'], b['elapsed_s']-a['elapsed_s']
        require(0 < wall <= MAX_GAP_SECONDS and elapsed > 0, 'missing/nonmonotonic five-second coverage')
        require(abs(wall-elapsed) <= 1, 'wall clock changed relative to sampler monotonic time')
    intervals = [b['elapsed_s']-a['elapsed_s'] for a,b in zip(coverage,coverage[1:])]
    require(intervals and 4.5 <= median(intervals) <= 5.5, 'sampler cadence is not five seconds')
    live = [r for r in samples if begin <= r['time'] <= end]
    require(live and live[0]['time']-begin <= MAX_GAP_SECONDS and end-live[-1]['time'] <= MAX_GAP_SECONDS,
            'memory does not cover game lifetime endpoints')

    previous_end = begin
    original_gravity = None
    phase_readbacks = {}
    for name in selected_phases:
        a, b = bounds[name]['Start'], bounds[name]['End']
        require(previous_end <= a < b <= end, 'unordered/out-of-lifetime phase: '+name)
        marks = [e for e in events if e.get('kind') == 'scenario' and e.get('name') == name]
        require(len(marks) == 2 and [e['mark'] for e in marks] == ['start', 'end'],
                'missing/duplicate event phase markers: '+name)
        if partial:
            players = [e for e in events if e.get('kind') == 'player'
                       and marks[0]['frame'] <= e['frame'] <= marks[1]['frame']]
            require(len(players) >= 3, 'missing healthy phase readbacks')
            actor = str(distance['lives'][0])
            gravity = players[0]['vitals'].get('gravity_scale', 0)
            require(gravity > 0 and (original_gravity is None or gravity == original_gravity),
                    'phase gravity unavailable/changed')
            original_gravity = gravity
            for e in players:
                v = e.get('vitals', {})
                require(str(v.get('entity')) == actor and v.get('health') == 100
                        and v.get('gravity_scale') == gravity and not e.get('picker')
                        and not e.get('map_open')
                        and all(math.isfinite(e[k]) for k in ('x','y','z','t')),
                        'unhealthy/changed phase actor')
            require(0 <= players[0]['t']-marks[0]['t'] <= 1.5 and 0 <= marks[1]['t']-players[-1]['t'] <= 1.5
                    and all(0 < q['t']-p['t'] <= 1.5 for p,q in zip(players,players[1:])),
                    'missing fresh phase readbacks')
            final_vitals=players[-1]['vitals']
            require(final_vitals.get('grounded') is True and final_vitals.get('pending_support') is False
                    and final_vitals.get('awaiting_surface') is False, 'phase final support unavailable')
            deaths=[e for e in events if e.get('kind') == 'died'
                    and marks[0]['frame'] <= e['frame'] <= marks[1]['frame']]
            require(not any(str(e.get('actor',{}).get('entity')) == actor for e in deaths),
                    'player death within admitted phase')
            phase_readbacks[name]=dict(actor=actor,player_samples=len(players),minimum_health=100,
                                       gravity_scale=gravity,final_supported=True,
                                       unrelated_death_events=len(deaths))
        previous_end = b

    def window(name, a, b, scope):
        chosen = [r for r in live if a <= r['time'] < b]
        require(chosen, 'no samples in '+name)
        require(chosen[0]['time']-a <= MAX_GAP_SECONDS and b-chosen[-1]['time'] <= MAX_GAP_SECONDS,
                'missing phase endpoint coverage: '+name)
        if b-a >= 15:
            require(len(chosen) >= 3, 'insufficient phase samples: '+name)
        counters = {}
        for key in ('phys_footprint_bytes', 'rss_bytes'):
            peak = max(chosen, key=lambda r:r[key])
            counters[key] = dict(median_bytes=median(r[key] for r in chosen), peak_bytes=peak[key],
                                 peak_utc=peak['utc'])
        return dict(scope=scope, coverage_status='covered', metrics_available=True, start_utc=datetime.fromtimestamp(a, timezone.utc).isoformat(),
                    end_utc=datetime.fromtimestamp(b, timezone.utc).isoformat(), duration_s=b-a,
                    sample_count=len(chosen), **counters)

    windows = {}
    for name in selected_phases:
        a,b=bounds[name]['Start'],bounds[name]['End']
        try:
            windows[name]=window(name,a,b,'ordinary walk' if name=='hitch-walk' else name)
        except ValueError as error:
            if name not in ('living-map','respawn'):
                raise
            windows[name]=dict(scope=name,coverage_status='unavailable',metrics_available=False,
                               reason=str(error),duration_s=b-a,sample_count=sum(a<=r['time']<b for r in live),
                               phys_footprint_bytes=None,rss_bytes=None)

    if partial:
        for name in ('living-map','respawn','whole-process'):
            windows[name]=dict(scope=name,coverage_status='outside-admitted-scope',metrics_available=False,
                               reason='partial settled/walk analysis only',sample_count=0,
                               phys_footprint_bytes=None,rss_bytes=None)
        windows['admitted-gameplay-epoch']=window('admitted-gameplay-epoch',begin,end,
                                                 'closed settled-to-walk epoch; partial run')
    else:
        windows['whole-process'] = window('whole-process', begin, end, 'includes startup, screenshots, map and death')
    checkpoints = []
    for index in (52, 104):
        rows = [e for e in life if e.get('kind') == 'pilot_checkpoint' and e.get('index') == index]
        require(len(rows) == 1, 'missing/duplicate revisit checkpoint '+str(index))
        e = rows[0]; t = utc(e['utc'])
        a, b = t-REVISIT_HALF_WINDOW_SECONDS, t+REVISIT_HALF_WINDOW_SECONDS
        require(bounds['hitch-walk']['Start'] <= a < b <= bounds['hitch-walk']['End'],
                'revisit window falls outside walk')
        nearest = min(live, key=lambda r:abs(r['time']-t))
        require(abs(nearest['time']-t) <= 3, 'no nearby five-second revisit sample')
        player = e['player']; v = player.get('vitals', {})
        supported=(v.get('health') == 100 and v.get('grounded') is True and
                   v.get('pending_support') is False and v.get('awaiting_surface') is False)
        checkpoints.append(dict(index=index, utc=e['utc'], x=player['x'], y=player['y'], z=player['z'],
                                yaw=player.get('yaw'), pitch=player.get('pitch'), actor=v.get('entity'),
                                nearest_sample=nearest, sample_offset_s=nearest['time']-t,
                                actual_camera='not observed at this checkpoint',ordinary_support_observed=supported))
        windows['revisit-'+str(index)] = window('revisit-'+str(index), a, b, 'moving +/-10s; not settled return')
    require(checkpoints[0]['actor'] is not None and checkpoints[0]['actor'] == checkpoints[1]['actor'],
            'checkpoint actor mismatch')
    distance = math.hypot(checkpoints[0]['x']-checkpoints[1]['x'], checkpoints[0]['z']-checkpoints[1]['z'])
    inputs = ['binding.json','collector-game-pid.json','memory.csv','lifecycle.jsonl','events.jsonl','game.log']
    inputs.append('distance.json' if partial else 'completed.json')
    source_binding = None
    if partial:
        source_binding = dict(expected_binding_sha256=binding_sha256, binary_sha256=binding['binary_sha256'],
                              manifest_sha256=binding['manifest_sha256'],
                              termination_sha256=sha(termination_receipt),
                              termination_exit_status=termination.get('exit_status'))
    return dict(accepted=True, analysis_scope=scope, partial_run=partial, source_binding=source_binding, phase_readbacks=phase_readbacks, pid=pid, process_start_abstime=live[0]['process_start_abstime'],
                configured_cadence_s=5, observed_median_cadence_s=median(intervals), maximum_accepted_gap_s=MAX_GAP_SECONDS,
                maximum_observed_gap_s=max((b['time']-a['time'] for a,b in zip(live,live[1:])), default=0),
                windows=windows, revisit_checkpoints=checkpoints, revisit_xz_separation_m=distance,
                revisit_within_10m=distance <= 10, inputs_sha256={n:sha(run/n) for n in inputs},
                note='Sampled process memory only; no ownership, leak or savings claim. Five-second cadence misses brief peaks. Revisit camera/content/quiescence unproven; map/death separated. No trace exports.')


def table(result):
    rows = ['| Window | Samples | Footprint median / peak MiB | RSS median / peak MiB |',
            '|---|---:|---:|---:|']
    for name in ('settled','hitch-walk','revisit-52','revisit-104','living-map','respawn','whole-process'):
        r=result['windows'][name]
        pair=lambda key:(f"{r[key]['median_bytes']/2**20:.2f} / {r[key]['peak_bytes']/2**20:.2f}"
                          if r['metrics_available'] else ('unavailable (outside scope)' if r['coverage_status']=='outside-admitted-scope'
                                else 'unavailable (no coverage)'))
        rows.append(f"| {name} | {r['sample_count']} | {pair('phys_footprint_bytes')} | {pair('rss_bytes')} |")
    if result.get('partial_run'):
        r=result['windows']['admitted-gameplay-epoch']
        rows.append(f"| admitted-gameplay-epoch (partial) | {r['sample_count']} | "
                    f"{r['phys_footprint_bytes']['median_bytes']/2**20:.2f} / {r['phys_footprint_bytes']['peak_bytes']/2**20:.2f} | "
                    f"{r['rss_bytes']['median_bytes']/2**20:.2f} / {r['rss_bytes']['peak_bytes']/2**20:.2f} |")
    label='Partial run: closed settled/walk epochs only; later lifecycle not admitted.\n\n' if result.get('partial_run') else ''
    return label+'\n'.join(rows)+'\n\n'+result['note']+'\n'


def main():
    cli=argparse.ArgumentParser(description=__doc__);cli.add_argument('run',type=Path);cli.add_argument('output',type=Path)
    cli.add_argument('--scope',choices=('whole-run','settled-walk'),default='whole-run')
    cli.add_argument('--binding-sha256');cli.add_argument('--termination-receipt',type=Path)
    args=cli.parse_args();args.output.mkdir(exist_ok=False)
    try:
        result=summarize(args.run,args.scope,args.binding_sha256,args.termination_receipt)
    except (ValueError,KeyError,OSError,json.JSONDecodeError) as error:
        (args.output/'rejected.json').write_text(json.dumps(dict(accepted=False,error=str(error)),indent=2)+'\n')
        raise SystemExit(str(error))
    (args.output/'summary.json').write_text(json.dumps(result,indent=2)+'\n')
    (args.output/'TABLE.md').write_text(table(result))


if __name__ == '__main__':
    main()

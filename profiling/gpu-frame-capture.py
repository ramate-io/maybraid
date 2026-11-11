#!/usr/bin/env python3
"""Capture one warm foreground render update; never use this run for timings."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import struct
import subprocess
import tempfile
import time


def read_json(path):
    try:
        return json.loads(path.read_text())
    except (FileNotFoundError, json.JSONDecodeError):
        return {}


def compile_foreground_helper(run):
    source = Path(__file__).with_name('gpu-frame-foreground.swift')
    helper = run / 'foreground-helper'
    command = ['/usr/bin/xcrun', 'swiftc', '-O', str(source), '-o', str(helper)]
    result = subprocess.run(command, text=True, capture_output=True, timeout=60)
    (run / 'foreground-helper-build.json').write_text(json.dumps({
        'command': command, 'returncode': result.returncode,
        'stdout': result.stdout, 'stderr': result.stderr,
        'source_sha256': hashlib.sha256(source.read_bytes()).hexdigest(),
    }, indent=2))
    if result.returncode:
        raise RuntimeError(f'foreground helper compilation failed: {result.stderr}')
    return helper


def foreground(pid, helper):
    try:
        result = subprocess.run([str(helper), str(pid)], text=True,
                                capture_output=True, timeout=5)
    except subprocess.TimeoutExpired as error:
        def captured_text(value):
            return value.decode(errors='replace') if isinstance(value, bytes) else value
        return {'frontmost': False, 'error': 'foreground helper timeout',
                'helper_stdout': captured_text(error.stdout),
                'helper_stderr': captured_text(error.stderr), 'helper_returncode': None}
    try:
        value = json.loads(result.stdout)
        if not isinstance(value, dict):
            raise ValueError('helper JSON is not an object')
    except (json.JSONDecodeError, ValueError):
        value = {'frontmost': False, 'error': 'invalid helper JSON'}
    value.update(helper_returncode=result.returncode, helper_stdout=result.stdout,
                 helper_stderr=result.stderr)
    if result.returncode or value.get('application_active') is not True or value.get('frontmost_pid') != pid:
        value['frontmost'] = False
    return value


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', required=True, type=Path, help='immutable feature-enabled binary')
    parser.add_argument('--manifest', required=True, type=Path,
                        help='JSON with source_head, binary_sha256, build_command, features')
    parser.add_argument('--assets', required=True, type=Path)
    parser.add_argument('--output-parent', required=True, type=Path)
    parser.add_argument('--script', type=Path, help='optional script containing gpu-frame-ready start mark')
    parser.add_argument('--start-at', default='976.6,1819.9')
    parser.add_argument('--timeout', type=float, default=180)
    args = parser.parse_args()
    binary = args.binary.resolve(strict=True)
    manifest = read_json(args.manifest)
    digest = hashlib.sha256(binary.read_bytes()).hexdigest()
    if not all(manifest.get(key) for key in ('source_head', 'binary_sha256', 'build_command', 'features')):
        parser.error('incomplete immutable binary manifest')
    if manifest['binary_sha256'] != digest or 'gpu-frame-capture' not in str(manifest['features']):
        parser.error('binary hash or gpu-frame-capture feature manifest mismatch')
    for name in ('maybraid', 'tracy', 'tracy-capture', 'tracy-profiler', 'xctrace', 'gpucapture', 'cargo', 'rustc'):
        if subprocess.run(['/usr/bin/pgrep', '-x', name], capture_output=True).returncode == 0:
            parser.error(f'{name} already active; this host is one-at-a-time')
    parent = args.output_parent.resolve()
    parent.mkdir(parents=True, exist_ok=True)
    run = Path(tempfile.mkdtemp(prefix='gpu-frame-', dir=parent)).resolve()
    for name in ('saves', 'logs'):
        (run / name).mkdir(mode=0o700)
    script_path = run / 'script.jsonl'
    if args.script:
        script = args.script.read_text().replace('@RUN_DIR@', str(run))
    else:
        commands = [
            {'cmd': 'console', 'line': 'sky noon'},
            {'cmd': 'wait', 'secs': 30},
            {'cmd': 'screenshot', 'path': str(run / 'ready.png')},
            {'cmd': 'scenario', 'name': 'gpu-frame-ready', 'mark': 'start'},
            {'cmd': 'wait', 'secs': 60},
            {'cmd': 'quit'},
        ]
        script = '\n'.join(json.dumps(command) for command in commands) + '\n'
    script_path.write_text(script)
    env = os.environ.copy()
    for key in ('MAYBRAID_AGENT_PIPE', 'MAYBRAID_START_AT', 'MAYBRAID_NO_START_AT'):
        env.pop(key, None)
    env.update(MTL_CAPTURE_ENABLED='1', MAYBRAID_GPU_CAPTURE_OUTPUT=str(run / 'frame.gputrace'),
               MAYBRAID_GPU_CAPTURE_TRIGGER=str(run / 'trigger'), MAYBRAID_VSYNC='off',
               MAYBRAID_FULLSCREEN='1', MAYBRAID_RESOLUTION='1920x1080',
               MAYBRAID_VISUAL_STYLE='native', MAYBRAID_RENDER_SCALE='1',
               MAYBRAID_SHADOWS='high', MAYBRAID_CANOPY_SHADOWS='proxy',
               MAYBRAID_AGENT_EXCLUSIVE='1', MAYBRAID_SAVES=str(run / 'saves'),
               MAYBRAID_LOGS=str(run / 'logs'), MAYBRAID_ASSETS=str(args.assets.resolve(strict=True)))
    helper = compile_foreground_helper(run)
    source_commit = subprocess.run(['git', '-C', str(Path(__file__).resolve().parent), 'rev-parse', 'HEAD'],
                                   capture_output=True, text=True, timeout=5)
    provenance = dict(manifest, binary=str(binary), binary_sha256=digest,
                      harness_source_commit=source_commit.stdout.strip() if source_commit.returncode == 0 else None,
                      harness_path=str(Path(__file__).resolve()),
                      foreground_helper_source_sha256=hashlib.sha256(Path(__file__).with_name('gpu-frame-foreground.swift').read_bytes()).hexdigest(),
                      harness_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                      foreground_helper_sha256=hashlib.sha256(helper.read_bytes()).hexdigest(),
                      script_sha256=hashlib.sha256(script.encode()).hexdigest(),
                      start_at=args.start_at, env={k: v for k, v in env.items() if k.startswith(('MAYBRAID_', 'MTL_'))},
                      scope='one render update; diagnostic only; device background work may appear',
                      start_utc=time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime()))
    (run / 'provenance.json').write_text(json.dumps(provenance, indent=2))
    print(f'RUN={run}', flush=True)
    process = None
    wake = None
    try:
        with (run / 'game.log').open('x') as log:
            process = subprocess.Popen([str(binary), '--no-supervise', f'--start-at={args.start_at}',
                                        '--agent-script', str(script_path), '--agent-events', str(run / 'events.jsonl'),
                                        '--agent-quit'], env=env, stdout=log, stderr=subprocess.STDOUT)
            provenance['pid'] = process.pid
            wake = subprocess.Popen(['/usr/bin/caffeinate', '-disu', '-w', str(process.pid)])
            provenance['wake_assertion_pid'] = wake.pid
            print(f'GAME_PID={process.pid}\nWAKE_PID={wake.pid}', flush=True)
            deadline = time.monotonic() + args.timeout
            triggered = False
            foreground_seen = False
            captured = False
            while time.monotonic() < deadline:
                if process.poll() is not None:
                    if captured and process.returncode == 0:
                        provenance['result'] = 'captured; clean game exit; Xcode inspection still required'
                        print('GAME_EXIT=0', flush=True)
                        return
                    raise RuntimeError(f'game exited before accepted capture/clean exit: {process.returncode}')
                front = foreground(process.pid, helper)
                with (run / 'foreground.jsonl').open('a') as log_front:
                    log_front.write(json.dumps({'utc': time.time(), 'pid': process.pid, **front}) + '\n')
                if not front['frontmost']:
                    if foreground_seen and not captured:
                        raise RuntimeError('foreground readback failed after initial acquisition')
                    time.sleep(1)
                    continue
                foreground_seen = True
                events_path = run / 'events.jsonl'
                events = []
                if events_path.exists():
                    for line in events_path.read_text().splitlines():
                        try:
                            events.append(json.loads(line))
                        except json.JSONDecodeError:
                            pass
                if any(e.get('status') == 'failed' for e in events):
                    raise RuntimeError('agent command failed; inspect events')
                status = read_json(run / 'capture-status.json')
                if status.get('state') in ('failed', 'aborted'):
                    raise RuntimeError(f'capture refused: {status}')
                ready = any(e.get('kind') == 'ready' and e.get('detail') == 'surface' for e in events)
                warm = any(e.get('name') == 'gpu-frame-ready' and e.get('mark') == 'start' for e in events)
                screenshot = run / 'ready.png'
                if not triggered and ready and warm and screenshot.exists() and status.get('state') == 'armed':
                    if (front.get('window_owner_pid') != process.pid or front.get('window_layer') != 0
                            or front.get('window_onscreen') is not True):
                        raise RuntimeError(f'target on-screen window ownership not verified: {front}')
                    if (front['window_width'], front['window_height']) != (1920, 1080):
                        raise RuntimeError(f'fullscreen dimensions differ from native target: {front}')
                    png = screenshot.read_bytes()
                    if png[:8] != b'\x89PNG\r\n\x1a\n' or struct.unpack('>II', png[16:24]) != (1920, 1080):
                        raise RuntimeError('ready screenshot is not native 1920x1080 PNG')
                    with (run / 'trigger').open('x') as trigger:
                        trigger.write(status['request_id'] + '\n')
                    triggered = True
                if status.get('state') == 'complete' and not captured:
                    if status['started_render_frame'] != status['stopped_render_frame']:
                        raise RuntimeError('capture spanned multiple render updates')
                    trace = run / 'frame.gputrace'
                    if not trace.is_dir() or not any(trace.iterdir()):
                        raise RuntimeError('capture stopped without a nonempty GPU trace document')
                    captured = True
                    provenance['result'] = 'captured; awaiting clean game exit and Xcode inspection'
                    print(f'CAPTURE_COMPLETE={trace}', flush=True)
                time.sleep(1)
            raise TimeoutError('bounded readiness/capture timeout')
    except BaseException as error:
        provenance['result'] = 'failed'
        provenance['error'] = str(error)
        raise
    finally:
        if process is not None and process.poll() is None:
            process.terminate()
            try:
                process.wait(timeout=10)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait(timeout=5)
        if wake is not None and wake.poll() is None:
            wake.terminate()
            try:
                wake.wait(timeout=5)
            except subprocess.TimeoutExpired:
                wake.kill()
                wake.wait(timeout=5)
        provenance['game_exit_status'] = None if process is None else process.returncode
        provenance['end_utc'] = time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime())
        (run / 'provenance.json').write_text(json.dumps(provenance, indent=2))


if __name__ == '__main__':
    main()

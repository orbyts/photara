#!/usr/bin/env python3
"""Supervised real subprocess/socket furnace. Fixtures never contact a provider."""
import base64
import contextlib
import json
import os
from pathlib import Path
import selectors
import signal
import socket
import subprocess
import sys
import time

root, operator, launcher = map(Path, sys.argv[1:])
operator_bin = operator / 'Contents/MacOS/photara-development-service'
service_bin = operator / 'Contents/Resources/photara-service'
checks = 0
scenarios = []
processes = []


def check(value, label):
    global checks
    if not value:
        raise AssertionError(label)
    checks += 1


def wait_for(predicate, label, timeout=12):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if predicate():
            return
        time.sleep(.02)
    raise AssertionError('Timed out: ' + label)


def alive(pid):
    try:
        os.kill(pid, 0)
        return True
    except ProcessLookupError:
        return False


def events(case):
    path = Path(case['events'])
    return [line.split() for line in path.read_text().splitlines()] if path.exists() else []


def saw(case, name):
    return any(row[1] == name for row in events(case))


def case(name, **changes):
    directory = root / name
    directory.mkdir()
    with socket.socket() as reserve:
        reserve.bind(('127.0.0.1', 0))
        port = reserve.getsockname()[1]
    check(port != 8080, 'Fixture must never use the live port')
    value = dict(port=port, events=str(directory / 'events'), lock=str(directory / 'startup.lock'),
                 cancel=str(directory / 'cancel'), budget=7.0)
    value.update(changes)
    return save(value, directory / 'case.json')


def save(value, path):
    value = dict(value, path=str(path))
    path.write_text(json.dumps(value))
    return value


def spawn(args, env, pipes=False):
    process = subprocess.Popen(list(map(str, args)), env=env, start_new_session=True,
                               stdin=subprocess.PIPE if pipes else subprocess.DEVNULL,
                               stdout=subprocess.PIPE if pipes else subprocess.DEVNULL,
                               stderr=subprocess.DEVNULL, bufsize=0)
    processes.append(process)
    return process


def direct(value):
    env = {'PATH': '/usr/bin:/bin', 'PHOTARA_FURNACE_CONFIG': value['path'],
           'PHOTARA_FORBIDDEN_INHERITANCE': 'must-be-stripped'}
    return spawn([operator_bin, 'run', service_bin], env)


def client(value):
    return spawn([launcher, value['path'], operator], {'PATH': '/usr/bin:/bin'}, pipes=True)


def send(process, command):
    process.stdin.write((command + '\n').encode())


def result(process, timeout=15):
    data = bytearray()
    deadline = time.monotonic() + timeout
    with selectors.DefaultSelector() as selector:
        selector.register(process.stdout, selectors.EVENT_READ)
        while time.monotonic() < deadline:
            if not selector.select(max(0, deadline - time.monotonic())):
                break
            byte = os.read(process.stdout.fileno(), 1)
            if byte == b'\n':
                return json.loads(data)
            if not byte:
                raise AssertionError('Launcher exited before its fixed result')
            data.extend(byte)
    raise AssertionError('Launcher result deadline exceeded')


def stop_client(process):
    send(process, 'stop')
    value = result(process, 4)
    check(value['status'] == 'stopped', 'Controller cleanup completed')
    process.wait(timeout=4)
    check(process.returncode == 0, 'Launcher exited cleanly')


def terminate(process):
    if process.poll() is None:
        os.killpg(process.pid, signal.SIGTERM)
        try:
            process.wait(timeout=2)
        except subprocess.TimeoutExpired:
            os.killpg(process.pid, signal.SIGKILL)
            process.wait(timeout=2)


def cleaned(value):
    for pid in {int(row[0]) for row in events(value)}:
        wait_for(lambda: not alive(pid), 'No fixture service remains', 4)
        check(not alive(pid), 'Fixture service PID is gone')
    with socket.socket() as probe:
        probe.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        probe.bind(('127.0.0.1', value['port']))
    check(True, 'Fixture port released')


def completed(name):
    scenarios.append(name)
    print('service-furnace:pass:' + name, flush=True)


values = {name: f'postgresql://{user}:synthetic-furnace@synthetic-furnace.neon.tech/neondb?sslmode=verify-full'
          for name, user in [('PHOTARA_DB_API_URL', 'photara_dev_api'), ('PHOTARA_DB_CONTROL_URL', 'photara_dev_control'),
                             ('PHOTARA_DB_AUTH_READ_URL', 'photara_dev_auth_read')]}
values['PHOTARA_CURSOR_KEY_B64'] = base64.urlsafe_b64encode(bytes([7]) * 32).decode().rstrip('=')
try:
    stored = subprocess.run([operator_bin, 'store'], input=json.dumps(values).encode(), stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=10)
    check(stored.returncode == 0, 'Signed fixture operator stores only synthetic configuration')

    c = case('cold-start')
    p = client(c); send(p, 'ensure'); r = result(p)
    check(r['status'] == 'ready' and r['launches'] == 1 and saw(c, 'bound'), 'Cold start through signed operator: ' + json.dumps(r))
    check(saw(c, 'health') and saw(c, 'capabilities'), 'Real URLSession crosses both HTTP routes')
    stop_client(p); cleaned(c); completed('cold-start')

    for name, options in [('healthy-reuse', {}), ('slow-health', {'delayHealth': 1.6}), ('transient-readiness-timeout', {'firstHealthDelay': 4.0})]:
        c = case(name, **options); external = direct(c)
        wait_for(lambda: saw(c, 'bound'), 'Existing service bound')
        p = client(c); send(p, 'ensure'); r = result(p)
        check(r['status'] == 'ready' and r['launches'] == 0, 'Healthy/slow reuse never duplicates')
        if name == 'transient-readiness-timeout':
            check(sum(row[1] == 'health' for row in events(c)) >= 2, 'Timed-out readiness is retried without relaunch')
        stop_client(p)
        check(external.poll() is None, 'Reused service untouched by controller cleanup')
        terminate(external); cleaned(c); completed(name)

    c = case('delayed-readiness')
    c['readyFile'] = str(Path(c['path']).with_name('ready')); c = save(c, Path(c['path']))
    external = direct(c); wait_for(lambda: saw(c, 'bound'), 'Delayed service bound')
    p = client(c); send(p, 'ensure')
    wait_for(lambda: sum(row[1] == 'health' for row in events(c)) >= 2, 'Multiple warming probes')
    Path(c['readyFile']).touch(); r = result(p)
    check(r['status'] == 'ready' and r['launches'] == 0, 'Warming authority is observed without duplicate launch')
    stop_client(p); terminate(external); cleaned(c); completed('delayed-readiness')

    c = case('concurrent-launchers'); one, two = client(c), client(c)
    send(one, 'ensure'); send(two, 'ensure'); a, b = result(one), result(two)
    check(a['status'] == b['status'] == 'ready', 'Both independent launcher processes converge')
    check(a['launches'] + b['launches'] == 1, 'Cross-process startup lock permits one launch')
    check(sum(row[1] == 'started' for row in events(c)) == 1, 'Exactly one operator exec service')
    stop_client(one); stop_client(two); cleaned(c); completed('concurrent-launchers')

    c = case('bind-race')
    c['bindFile'] = str(Path(c['path']).with_name('bind')); c = save(c, Path(c['path']))
    p = client(c); send(p, 'ensure'); wait_for(lambda: saw(c, 'started'), 'Owned child waiting before bind')
    winner = dict(c, bindFile=None, readyFile=str(Path(c['path']).with_name('ready')))
    winner = save(winner, Path(c['path']).with_name('winner.json'))
    external = direct(winner); wait_for(lambda: saw(c, 'bound'), 'Competing authority binds')
    Path(c['bindFile']).touch(); wait_for(lambda: saw(c, 'exit-bind-race'), 'Owned child loses bind and exits')
    Path(winner['readyFile']).touch(); r = result(p)
    check(r['status'] == 'ready' and r['launches'] == 1, 'Exited child cannot reject another valid authority')
    stop_client(p); check(external.poll() is None, 'Bind winner retained')
    terminate(external); cleaned(c); completed('child-exit-with-valid-bind-winner')

    for name, options in [('wrong-service', {'wrongCapabilities': True}), ('wrong-health', {'wrongHealth': True})]:
        c = case(name, **options); external = direct(c); wait_for(lambda: saw(c, 'bound'), 'Wrong service bound')
        p = client(c); send(p, 'ensure'); r = result(p)
        check(r['status'] == 'failed' and r['launches'] == 0, 'Wrong protocol fails without a duplicate launch')
        stop_client(p); check(external.poll() is None, 'Unowned process is never killed')
        terminate(external); cleaned(c); completed(name)

    c = case('exited-no-authority', exitBeforeBind=True, budget=3.0)
    p = client(c); send(p, 'ensure'); r = result(p)
    check(r['status'] == 'failed' and r['launches'] == 1 and saw(c, 'exit-before-bind'), 'Exited child without authority fails')
    check(r['seconds'] < 5, 'No-authority failure is bounded')
    stop_client(p); cleaned(c); completed('child-exit-without-authority')

    c = case('cancel-startup', ignoreTerm=True)
    c['bindFile'] = str(Path(c['path']).with_name('never-bind')); c = save(c, Path(c['path']))
    p = client(c); send(p, 'ensure'); wait_for(lambda: saw(c, 'started'), 'Cancelable child started')
    begin = time.monotonic(); Path(c['cancel']).touch(); r = result(p)
    check(r['status'] == 'cancelled' and r['forcedStops'] == 1, 'Task cancellation escalates TERM to KILL for its child')
    check(time.monotonic() - begin < 4, 'Cancellation and reaping bounded')
    stop_client(p); cleaned(c); completed('cancellation-and-uncooperative-child-cleanup')

    c = case('crash-restart'); p = client(c); send(p, 'ensure'); first = result(p)
    check(first['status'] == 'ready' and first['launches'] == 1, 'Initial service is ready')
    os.kill(first['pid'], signal.SIGKILL)
    wait_for(lambda: not alive(first['pid']), 'Crashed service reaped')
    send(p, 'ensure'); second = result(p)
    check(second['status'] == 'ready' and second['launches'] == 2 and second['pid'] != first['pid'], 'Same controller restarts crashed service')
    stop_client(p); cleaned(c); completed('service-crash-and-restart')

    c = case('never-ready', budget=2.5)
    c['readyFile'] = str(Path(c['path']).with_name('never-ready')); c = save(c, Path(c['path']))
    external = direct(c); wait_for(lambda: saw(c, 'bound'), 'Unready service bound')
    p = client(c); send(p, 'ensure'); r = result(p)
    check(r['status'] == 'failed' and r['launches'] == 0 and r['seconds'] < 4.5, 'Unready service times out without duplication')
    stop_client(p); check(external.poll() is None, 'Unready unowned service untouched')
    terminate(external); cleaned(c); completed('bounded-unready-service')

    denied = subprocess.run([operator_bin, 'run', '/private/tmp/photara-service'], stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=5)
    check(denied.returncode != 0 and denied.stderr == b'development-service:service-path-not-embedded\n', 'Operator refuses nonembedded child before reading credentials')
finally:
    for process in processes:
        with contextlib.suppress(ProcessLookupError, subprocess.TimeoutExpired):
            # Group cleanup also catches a fixture child if a launcher crashes.
            os.killpg(process.pid, signal.SIGKILL)
            process.wait(timeout=3)
    erased = subprocess.run([operator_bin, 'erase-fixture'], stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=10)
    check(erased.returncode == 0, 'Synthetic operator Keychain item removed')
report = {'assertions': checks, 'scenarios': scenarios, 'production_port_used': False, 'providers_used': False}
(root / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
print(f'service-process-furnace: {checks} assertions; {len(scenarios)} real-process scenarios passed; no fixture processes or credentials retained', flush=True)

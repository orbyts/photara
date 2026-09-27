"""Private bounded PostgreSQL relay for the opt-in LL1 commit-ACK fixture.

Only protocol metadata is recorded. Never print SQL, startup parameters, bind
values, backend cancellation secrets, credentials or receipt payloads.
"""
import json
import os
from pathlib import Path
import select
import socket
import struct
import threading
import time

COMMIT = b'Q\x00\x00\x00\x0bCOMMIT\x00'
FRAME_CAP = 1024 * 1024
STARTUP_CAP = 65536


def read_timeout(stream, deadline):
    remaining = deadline-time.monotonic()
    if remaining <= 0:
        raise ValueError('absolute-read-deadline')
    stream.settimeout(remaining)


def exact(stream, size, deadline):
    result = bytearray()
    while len(result) < size:
        read_timeout(stream, deadline)
        try:
            data = stream.recv(size-len(result))
        except socket.timeout:
            raise ValueError('absolute-read-deadline') from None
        if not data:
            raise EOFError
        result.extend(data)
    return bytes(result)


def frame(stream, deadline):
    header = exact(stream, 5, deadline)
    length = struct.unpack('!I', header[1:])[0]
    if not 4 <= length <= FRAME_CAP:
        raise ValueError('frame-size')
    return header+exact(stream, length-4, deadline)


def verify_absolute_read_deadline():
    """A trickled local stream must not restart the total receive budget."""
    reader, writer = socket.socketpair()
    stop = threading.Event()

    def trickle():
        try:
            for _ in range(100):
                writer.sendall(b'x')
                if stop.wait(0.01):
                    break
        except OSError:
            pass

    deadline = time.monotonic()+0.05
    worker = threading.Thread(target=trickle, daemon=True)
    worker.start()
    try:
        try:
            exact(reader, 100, deadline)
        except ValueError as error:
            assert str(error) == 'absolute-read-deadline'
        else:
            raise AssertionError('trickled read exceeded total budget')
    finally:
        stop.set()
        reader.close()
        writer.close()
        worker.join(timeout=2)
        assert not worker.is_alive()


class CommitRelay:
    def __init__(self, root, backend):
        self.root = Path(root).resolve(strict=True)
        assert str(self.root).startswith('/private/tmp/photara-ll1-constraints-')
        assert Path(backend).resolve(strict=True) == self.root/'socket'
        self.socket = self.root/'relay'
        self.socket.mkdir(mode=0o700)
        self.backend = str(Path(backend)/'.s.PGSQL.55439')
        self.control = self.root/'relay-control.sock'
        self.stop = threading.Event()
        self.lock = threading.Lock()
        self.sessions = {}
        self.streams = []
        self.threads = []
        self.errors = []
        self.accepted = 0
        self.controls = 0
        self.accept_counts = {'connection': 0, 'command': 0}
        self.listeners = []
        for path, handler in ((self.socket/'.s.PGSQL.55439', self.connection),
                              (self.control, self.command)):
            listener = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
            listener.bind(str(path))
            os.chmod(path, 0o600)
            listener.listen(8)
            listener.settimeout(0.2)
            self.listeners.append(listener)
            self.launch(self.accept, listener, handler)

    def launch(self, function, *args):
        worker = threading.Thread(target=function, args=args, daemon=True)
        self.threads.append(worker)
        worker.start()

    def accept(self, listener, handler):
        while not self.stop.is_set():
            try:
                stream, _ = listener.accept()
            except socket.timeout:
                continue
            except OSError:
                return
            with self.lock:
                name = handler.__name__
                cap = 16 if name == 'connection' else 256
                admitted = not self.stop.is_set() and self.accept_counts[name] < cap
                if admitted:
                    self.accept_counts[name] += 1
                    self.streams.append(stream)
                elif not self.stop.is_set() and 'accept-cap' not in self.errors:
                    self.errors.append('accept-cap')
            if not admitted:
                self.shutdown(stream)
                continue
            stream.settimeout(5)
            self.launch(handler, stream)

    @staticmethod
    def shutdown(stream):
        try:
            stream.shutdown(socket.SHUT_RDWR)
        except OSError:
            pass
        stream.close()

    def connection(self, client):
        server = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        server.settimeout(5)
        session = None
        end = time.monotonic()+90
        try:
            with self.lock:
                if self.stop.is_set():
                    return
                self.accepted += 1
                if self.accepted > 16:
                    raise ValueError('connection-cap')
                self.streams.append(server)
            server.connect(self.backend)
            for _ in range(4):
                deadline = min(end, time.monotonic()+5)
                header = exact(client, 4, deadline)
                length = struct.unpack('!I', header)[0]
                if not 8 <= length <= STARTUP_CAP:
                    raise ValueError('startup-size')
                payload = exact(client, length-4, deadline)
                code = struct.unpack('!I', payload[:4])[0]
                if code in (80877103, 80877104) and length == 8:
                    # Explicitly support only refusal of SSL/GSS negotiation on
                    # this disposable socket-only server, never opaque TLS.
                    server.sendall(header+payload)
                    response = exact(server, 1, deadline)
                    if response != b'N':
                        raise ValueError('unsupported-encrypted-startup')
                    client.sendall(response)
                    continue
                if code != 196608:
                    raise ValueError('unsupported-startup')
                server.sendall(header+payload)
                break
            else:
                raise ValueError('startup-negotiation-cap')
            while not self.stop.is_set() and time.monotonic() < end:
                ready, _, _ = select.select([client, server], [], [], 0.2)
                for source in ready:
                    packet = frame(source, min(end, time.monotonic()+5))
                    if source is client:
                        with self.lock:
                            mode = session['mode'] if session else None
                            state = session['state'] if session else None
                        if mode:
                            if state != 'armed' or packet != COMMIT:
                                raise ValueError('unexpected-armed-frontend-frame')
                            with self.lock:
                                session['commit_captured'] = True
                                if mode == 'before-forward':
                                    session['state'] = 'dropped-before-forward'
                                    return
                                session['state'] = 'commit-forwarded'
                                session['commit_forwarded'] = True
                        server.sendall(packet)
                    else:
                        if packet[:1] == b'K':
                            if len(packet) != 13 or session is not None:
                                raise ValueError('backend-key-frame')
                            # Only retain the PID, never the cancellation secret.
                            pid = struct.unpack('!I', packet[5:9])[0]
                            session = {'pid': pid, 'mode': None, 'state': 'connected',
                                       'commit_captured': False, 'commit_forwarded': False,
                                       'complete_seen': False, 'idle_seen': False,
                                       'withheld_frames': 0, 'ack_bytes_forwarded': 0,
                                       'client': client, 'server': server}
                            with self.lock:
                                if pid in self.sessions:
                                    raise ValueError('duplicate-backend-pid')
                                self.sessions[pid] = session
                        with self.lock:
                            hold = session and session['commit_forwarded']
                        if hold:
                            with self.lock:
                                session['withheld_frames'] += 1
                                if session['withheld_frames'] > 8:
                                    raise ValueError('commit-response-frame-cap')
                                if packet == b'C\x00\x00\x00\x0bCOMMIT\x00' and not session['complete_seen']:
                                    session['complete_seen'] = True
                                elif packet == b'Z\x00\x00\x00\x05I' and session['complete_seen']:
                                    session['idle_seen'] = True
                                    session['state'] = 'committed-ack-withheld'
                                else:
                                    raise ValueError('unexpected-commit-response')
                            # No byte of either C or Z reaches the SQLx client.
                            continue
                        client.sendall(packet)
            if not self.stop.is_set():
                raise ValueError('connection-lifetime-bound')
        except (EOFError, OSError):
            # Normal pool teardown and deliberate drop close both stream ends.
            pass
        except ValueError as error:
            with self.lock:
                if not self.stop.is_set() and not (session and session['state'] == 'dropped-after-commit'):
                    self.errors.append(str(error))
        finally:
            self.shutdown(client)
            self.shutdown(server)

    def command(self, control):
        try:
            with self.lock:
                self.controls += 1
                if self.controls > 256:
                    raise ValueError('control-count-cap')
            data = bytearray()
            deadline = time.monotonic()+5
            while b'\n' not in data:
                read_timeout(control, deadline)
                part = control.recv(4097-len(data))
                if not part or len(data)+len(part) > 4096:
                    raise ValueError('control-size')
                data.extend(part)
            request = json.loads(data)
            if set(request) != {'action', 'pid', 'mode'} or type(request['pid']) is not int:
                raise ValueError('control-shape')
            with self.lock:
                session = self.sessions.get(request['pid'])
                if session is None:
                    raise ValueError('unknown-backend-pid')
                action = request['action']
                if action == 'arm':
                    if session['state'] != 'connected' or request['mode'] not in ('before-forward', 'after-commit'):
                        raise ValueError('invalid-arm')
                    session['mode'] = request['mode']
                    session['state'] = 'armed'
                elif action == 'drop':
                    if session['state'] != 'committed-ack-withheld':
                        raise ValueError('drop-before-server-commit')
                    session['state'] = 'dropped-after-commit'
                    self.shutdown(session['client'])
                    self.shutdown(session['server'])
                elif action != 'status':
                    raise ValueError('control-action')
                response = {k: v for k, v in session.items() if k not in ('client', 'server')}
                response['errors'] = list(self.errors)
            control.sendall(json.dumps(response).encode()+b'\n')
        except (ValueError, KeyError, OSError):
            # Keep any details/payload out of diagnostics; the fixture must fail.
            with self.lock:
                self.errors.append('invalid-control-request')
            try:
                control.sendall(b'{"error":"invalid-control-request"}\n')
            except OSError:
                pass
        finally:
            self.shutdown(control)

    def proof(self):
        with self.lock:
            assert not self.errors, self.errors
            faults = [s for s in self.sessions.values() if s['mode']]
            assert len(faults) == 2
            assert {s['state'] for s in faults} == {'dropped-before-forward', 'dropped-after-commit'}
            assert all(s['commit_captured'] and s['ack_bytes_forwarded'] == 0 for s in faults)
            for session in faults:
                committed = session['mode'] == 'after-commit'
                assert session['commit_forwarded'] == committed
                assert session['complete_seen'] == session['idle_seen'] == committed
                assert session['withheld_frames'] == (2 if committed else 0)
            return [{k: v for k, v in s.items() if k not in ('client', 'server')} for s in faults]

    def close(self):
        self.stop.set()
        for listener in self.listeners:
            self.shutdown(listener)
        for stream in self.streams:
            self.shutdown(stream)
        for worker in self.threads:
            worker.join(timeout=2)
            assert not worker.is_alive(), 'private relay worker did not stop'

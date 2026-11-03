#!/usr/bin/env python3
"""Exercise the real daemon with simulated NetworkManager and curl, without touching Wi-Fi."""
import json, os, pathlib, subprocess, tempfile, time, unittest, sys
BINARY = pathlib.Path(sys.argv.pop(1)).resolve()
FAKE = r'''#!/usr/bin/env python3
import sys,os,json,pathlib,time
p=pathlib.Path(os.environ['SIM_ROOT']);a=sys.argv[1:];tool=pathlib.Path(sys.argv[0]).name
assert 'LD_LIBRARY_PATH' not in os.environ, 'AppImage libraries leaked into host tools'
assert 'LD_PRELOAD' not in os.environ
assert 'PYTHONHOME' not in os.environ
assert 'PYTHONPATH' not in os.environ
assert not any(x.startswith(os.environ['APPDIR']) for x in os.environ['PATH'].split(':'))
active=p.joinpath('active').read_text().strip()
with p.joinpath('calls').open('a') as f:f.write(tool+' '+repr(a)+'\n')
ids={'a':'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa','b':'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb','c':'cccccccc-cccc-cccc-cccc-cccccccccccc','o':'dddddddd-dddd-dddd-dddd-dddddddddddd'}
if tool=='nmcli':
 if a==['-t','-f','DEVICE,TYPE','device','status']: print('wlan-test:wifi')
 elif a[:2]==['-g','GENERAL.CON-UUID']: print(ids.get(active,'--'))
 elif a==['-t','-f','UUID,TYPE','connection','show']:
  print('\n'.join(v+':802-11-wireless' for v in ids.values()))
 elif 'wifi' in a and 'list' in a: print('Network A:50:2412 MHz:WPA2\nNetwork B:80:5180 MHz:WPA2\nNetwork C:70:2412 MHz:WPA2\nOpen:90:2412 MHz:--')
 elif a[:2]==['-g','connection.id,802-11-wireless.ssid']:
  letter=next(k for k,v in ids.items() if v==a[-1]);name='Open' if letter=='o' else 'Network '+letter.upper();print(name+'\n'+name)
 elif 'up' in a:
  letter=next(k for k,v in ids.items() if v==a[a.index('uuid')+1]);p.joinpath('active').write_text(letter);print('activated')
 elif 'disconnect' in a:p.joinpath('active').write_text('');print('disconnected')
 else:sys.exit(2)
else:
 assert a[a.index('--interface')+1]=='wlan-test'
 failures=p.joinpath('fail').read_text()
 if active in failures: print('503 0');sys.exit(0)
 if 'generate_204' in a[-1]:print('204 0.02')
 elif '__down' in a[-1]:print('200 2000000 0')
 elif '__up' in a[-1]:sys.stdin.buffer.read();print('200 1000000')
 else:sys.exit(2)
'''
class Integration(unittest.TestCase):
 def setUp(self):
  self.tmp=tempfile.TemporaryDirectory(prefix='swap-test-');self.root=pathlib.Path(self.tmp.name);self.config=self.root/'config';self.runtime=self.root/'runtime';self.runtime.mkdir();self.bin=self.root/'bin';self.bin.mkdir()
  for name in ['nmcli','curl']:
   f=self.bin/name;f.write_text(FAKE);f.chmod(0o755)
  for name,value in [('active','a'),('fail',''),('calls','')]: (self.root/name).write_text(value)
  self.env={**os.environ,'XDG_CONFIG_HOME':str(self.config),'XDG_RUNTIME_DIR':str(self.runtime),'SIM_ROOT':str(self.root),'PATH':str(self.root/'bundle/usr/bin')+':'+str(self.bin)+':'+os.environ['PATH'],'APPDIR':str(self.root/'bundle'),'LD_LIBRARY_PATH':str(self.root/'bundle/usr/lib'),'PYTHONHOME':str(self.root/'bundle/usr'),'PYTHONPATH':str(self.root/'bundle/usr/share/pyshared')}
  self.process=None;self.start()
 def start(self):
  self.process=subprocess.Popen([str(BINARY),'--daemon'],env=self.env,stderr=subprocess.PIPE)
  self.wait(lambda:self.status().get('networks'),8)
 def tearDown(self):
  if self.process:self.process.terminate();self.process.wait(timeout=5);self.process.stderr.close()
  self.tmp.cleanup()
 def command(self,flag,*args):
  p=subprocess.run([str(BINARY),flag,*args],env=self.env,capture_output=True,text=True,timeout=5)
  return json.loads(p.stdout) if p.stdout else {}
 def status(self):return self.command('--status')
 def wait(self,predicate,seconds=20):
  until=time.monotonic()+seconds
  while time.monotonic()<until:
   try:
    if predicate():return
   except (OSError,ValueError,KeyError):pass
   time.sleep(.15)
  self.fail('condition timed out; status='+repr(self.status()))
 def test_private_ipc_saved_secured_and_single_instance(self):
  self.assertEqual(len(self.status()['networks']),3)
  socket=self.runtime/'internet-swap/socket';self.assertEqual(socket.stat().st_mode&0o777,0o600)
  self.assertEqual(socket.parent.stat().st_mode&0o777,0o700)
  other=subprocess.run([str(BINARY),'--daemon'],env=self.env,timeout=4)
  self.assertEqual(other.returncode,0);self.assertTrue(self.status()['enabled'])
  result=self.command('--connect','not-a-profile');self.assertIn('error',result)
 def test_pause_persists_across_restart(self):
  self.assertTrue(self.command('--disable')['ok']);self.assertFalse(self.status()['enabled'])
  self.process.terminate();self.process.wait(timeout=5);self.process.stderr.close();self.start()
  self.assertFalse(self.status()['enabled']);time.sleep(2)
  before=(self.root/'calls').read_text().count('curl ');time.sleep(2);self.assertEqual(before,(self.root/'calls').read_text().count('curl '))
  self.assertIn('error',self.command('--test'));self.assertTrue(self.command('--enable')['ok'])
 def test_manual_test_restores_original_and_failed_tests_are_unknown(self):
  (self.root/'fail').write_text('b');self.command('--test')
  self.wait(lambda:all(n['measurement'] is not None for n in self.status()['networks']))
  self.wait(lambda:(self.root/'active').read_text()=='a' and self.status()['phase']=='watching')
  b=next(n for n in self.status()['networks'] if n['name']=='Network B')
  self.assertFalse(b['measurement']['healthy']);self.assertIsNone(b['measurement']['download'])
 def test_failed_candidate_rolls_back_then_recovers(self):
  (self.root/'fail').write_text('ab')
  self.wait(lambda:(self.root/'active').read_text()=='c' and self.status()['current'].startswith('cccc') and any('Switched to a working' in e['text'] for e in self.status()['events']),25)
  calls=(self.root/'calls').read_text()
  self.assertIn("'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb'",calls)
  self.assertIn("'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa'",calls)
  self.assertTrue(any('Switched to a working' in e['text'] for e in self.status()['events']))
if __name__=='__main__':unittest.main(verbosity=2)

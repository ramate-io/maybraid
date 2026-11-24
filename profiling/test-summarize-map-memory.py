"""Synthetic fixtures only; never reads native map-arm data."""
import csv
from datetime import datetime, timedelta, timezone
import hashlib
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

HERE=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('summary',HERE/'summarize-map-memory.py')
m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)


def fixture(root):
    origin=datetime(2026,1,1,tzinfo=timezone.utc)
    stamp=lambda seconds:(origin+timedelta(seconds=seconds)).isoformat()
    write=lambda name,data:(root/name).write_text(json.dumps(data)+'\n')
    write('binding.json',{'fixture':'synthetic-only'})
    write('completed.json',dict(normal_exit=True,distance_goal_reached=True,
        binding_sha256=hashlib.sha256((root/'binding.json').read_bytes()).hexdigest()))
    write('collector-game-pid.json',{'pid':1234})
    life=[dict(kind='game_launched',utc=stamp(0),pid=1234),dict(kind='game_exited',utc=stamp(200),exit_status=0)]
    for index,t in [(52,100),(104,150)]:
        life.append(dict(kind='pilot_checkpoint',utc=stamp(t),index=index,
            player=dict(x=10,y=0,z=10,yaw=0,pitch=0,vitals=dict(entity='42',health=100,
                grounded=True,pending_support=False,awaiting_surface=False))))
    (root/'lifecycle.jsonl').write_text(''.join(json.dumps(e)+'\n' for e in life))
    phases={'settled':(10,50),'hitch-walk':(60,170),'living-map':(175,185),'respawn':(185,195)}
    logs=[];events=[]
    for name,(a,b) in phases.items():
        for endpoint,t in [('Start',a),('End',b)]:
            logs.append(f'{stamp(t)} INFO maybraid.agent: agent.scenario {name} {endpoint}\n')
            events.append(dict(kind='scenario',name=name,mark=endpoint.lower(),t=t,frame=t*60))
    (root/'game.log').write_text(''.join(logs));(root/'events.jsonl').write_text(''.join(json.dumps(e)+'\n' for e in events))
    with (root/'memory.csv').open('w') as f:
        w=csv.writer(f);w.writerow(['utc','elapsed_s','pid','process_start_abstime','rss_bytes','phys_footprint_bytes'])
        for t in range(0,201,5):w.writerow([stamp(t),t,1234,777,50000+t,100000+t])


def partial_fixture(root):
    (root/'completed.json').unlink()
    binding={'binary_sha256':'b'*64,'manifest_sha256':'c'*64,
             'deadline_receipt':str(root/'termination.json')}
    (root/'binding.json').write_text(json.dumps(binding)+'\n')
    (root/'termination.json').write_text(json.dumps(dict(start_utc='2026-01-01T00:00:00+00:00',
        end_utc='2026-01-01T00:03:20+00:00',exit_status=1,owned_group_remaining=False))+'\n')
    (root/'distance.json').write_text(json.dumps(dict(goal_reached=True,active_xz_m=2500,
        minimum_health=100,lives=['42'],accepted_edges=500,rejected_edges=6))+'\n')
    life=[json.loads(l) for l in (root/'lifecycle.jsonl').read_text().splitlines()]
    (root/'lifecycle.jsonl').write_text(''.join(json.dumps(e)+'\n' for e in life if e['kind']!='game_exited'))
    events=[json.loads(l) for l in (root/'events.jsonl').read_text().splitlines()]
    events=[e for e in events if e['name'] in ('settled','hitch-walk')]
    for t2 in range(20,341):
        t=t2/2
        events.append(dict(kind='player',t=t,frame=t*60,x=1,y=0,z=1,
            vitals=dict(entity='42',health=100,gravity_scale=1.25,grounded=True,
                        pending_support=False,awaiting_surface=False)))
    events.sort(key=lambda e:(e['frame'],e['kind']!='scenario'))
    (root/'events.jsonl').write_text(''.join(json.dumps(e)+'\n' for e in events))
    log=[l for l in (root/'game.log').read_text().splitlines() if 'living-map' not in l and 'respawn' not in l]
    (root/'game.log').write_text('\n'.join(log)+'\n')
    return dict(scope='settled-walk',binding_sha256=m.sha(root/'binding.json'),
                termination_receipt=root/'termination.json')


class MemorySummary(unittest.TestCase):
    def setUp(self):
        self.tmp=tempfile.TemporaryDirectory();self.run=Path(self.tmp.name);fixture(self.run)
    def tearDown(self):self.tmp.cleanup()
    def edit_rows(self,fn):
        with (self.run/'memory.csv').open() as f:r=list(csv.reader(f))
        fn(r)
        with (self.run/'memory.csv').open('w') as f:csv.writer(f).writerows(r)
    def test_valid_phase_peak_and_revisit_table(self):
        r=m.summarize(self.run);self.assertTrue(r['accepted']);self.assertEqual(r['pid'],1234)
        self.assertEqual(r['windows']['settled']['sample_count'],8);self.assertEqual(r['windows']['settled']['phys_footprint_bytes']['peak_bytes'],100045)
        self.assertEqual(r['revisit_xz_separation_m'],0);self.assertIn('moving',r['windows']['revisit-52']['scope'])
        self.assertIn('whole-process',m.table(r))
    def test_short_observer_without_sample_is_explicitly_unavailable(self):
        p=self.run/'game.log';lines=p.read_text().splitlines()
        for i,line in enumerate(lines):
            if 'agent.scenario respawn ' in line:
                stamp='2026-01-01T00:03:05.100000+00:00' if line.endswith('Start') else '2026-01-01T00:03:09.400000+00:00'
                lines[i]=stamp+' '+line.split(' ',1)[1]
        p.write_text('\n'.join(lines)+'\n')
        p=self.run/'events.jsonl';events=[json.loads(l) for l in p.read_text().splitlines()]
        for e in events:
            if e['name']=='respawn':e['t']=185.1 if e['mark']=='start' else 189.4
        p.write_text(''.join(json.dumps(e)+'\n' for e in events))
        r=m.summarize(self.run);self.assertTrue(r['accepted']);self.assertTrue(r['windows']['hitch-walk']['metrics_available'])
        phase=r['windows']['respawn'];self.assertEqual(phase['coverage_status'],'unavailable')
        self.assertEqual(phase['sample_count'],0);self.assertIsNone(phase['phys_footprint_bytes'])
        self.assertIn('unavailable (no coverage)',m.table(r))
    def test_partial_closed_phases_without_completion_or_game_exit(self):
        args=partial_fixture(self.run);r=m.summarize(self.run,**args)
        self.assertTrue(r['partial_run']);self.assertTrue(r['windows']['hitch-walk']['metrics_available'])
        self.assertFalse(r['windows']['whole-process']['metrics_available'])
        self.assertIn('outside scope',m.table(r));self.assertFalse((self.run/'completed.json').exists())
        with self.assertRaises(FileNotFoundError):m.summarize(self.run)
    def test_partial_allows_unrelated_npc_death(self):
        args=partial_fixture(self.run);p=self.run/'events.jsonl'
        with p.open('a') as f:f.write(json.dumps(dict(kind='died',frame=6000,t=100,actor={'entity':'other'}))+'\n')
        r=m.summarize(self.run,**args)
        self.assertEqual(r['phase_readbacks']['hitch-walk']['unrelated_death_events'],1)
    def test_partial_rejects_player_death(self):
        args=partial_fixture(self.run);p=self.run/'events.jsonl'
        with p.open('a') as f:f.write(json.dumps(dict(kind='died',frame=6000,t=100,actor={'entity':'42'}))+'\n')
        with self.assertRaisesRegex(ValueError,'player death'):m.summarize(self.run,**args)
    def test_partial_rejects_unbound_source(self):
        args=partial_fixture(self.run);args['binding_sha256']='d'*64
        with self.assertRaisesRegex(ValueError,'binding digest mismatch'):m.summarize(self.run,**args)
    def test_partial_rejects_open_walk(self):
        args=partial_fixture(self.run);p=self.run/'game.log';p.write_text(p.read_text().replace('agent.scenario hitch-walk End','open'))
        with self.assertRaisesRegex(ValueError,'missing UTC phase'):m.summarize(self.run,**args)
    def test_partial_rejects_health_change(self):
        args=partial_fixture(self.run);p=self.run/'events.jsonl';es=[json.loads(l) for l in p.read_text().splitlines()]
        next(e for e in es if e['kind']=='player' and e['t']==100)['vitals']['health']=99
        p.write_text(''.join(json.dumps(e)+'\n' for e in es))
        with self.assertRaisesRegex(ValueError,'unhealthy'):m.summarize(self.run,**args)
    def test_partial_rejects_stale_readbacks(self):
        args=partial_fixture(self.run);p=self.run/'events.jsonl';es=[json.loads(l) for l in p.read_text().splitlines()]
        es=[e for e in es if not(e['kind']=='player' and 99<=e['t']<=102)]
        p.write_text(''.join(json.dumps(e)+'\n' for e in es))
        with self.assertRaisesRegex(ValueError,'fresh phase'):m.summarize(self.run,**args)
    def test_partial_rejects_failed_distance_goal(self):
        args=partial_fixture(self.run);p=self.run/'distance.json';d=json.loads(p.read_text());d['goal_reached']=False;p.write_text(json.dumps(d))
        with self.assertRaisesRegex(ValueError,'distance/healthy'):m.summarize(self.run,**args)
    def test_partial_rejects_unclean_termination(self):
        args=partial_fixture(self.run);p=self.run/'termination.json';d=json.loads(p.read_text());d['owned_group_remaining']=True;p.write_text(json.dumps(d))
        with self.assertRaisesRegex(ValueError,'cleanup unverified'):m.summarize(self.run,**args)
    def test_reject_wrong_pid(self):
        self.edit_rows(lambda r:r[5].__setitem__(2,'4321'))
        with self.assertRaisesRegex(ValueError,'PID mismatch'):m.summarize(self.run)
    def test_reject_process_reuse(self):
        self.edit_rows(lambda r:r[5].__setitem__(3,'778'))
        with self.assertRaisesRegex(ValueError,'identity changed'):m.summarize(self.run)
    def test_reject_missing_five_second_sample(self):
        self.edit_rows(lambda r:r.pop(5))
        with self.assertRaisesRegex(ValueError,'coverage'):m.summarize(self.run)
    def test_reject_truncated_tail(self):
        self.edit_rows(lambda r:r.__delitem__(slice(-3,None)))
        with self.assertRaisesRegex(ValueError,'lifetime endpoints'):m.summarize(self.run)
    def test_reject_missing_actual_utc_marker(self):
        p=self.run/'game.log';p.write_text(p.read_text().replace('agent.scenario settled End','not-a-marker'))
        with self.assertRaisesRegex(ValueError,'missing UTC phase'):m.summarize(self.run)
    def test_reject_mismatched_completion_binding(self):
        (self.run/'binding.json').write_text('{}\n')
        with self.assertRaisesRegex(ValueError,'binding mismatch'):m.summarize(self.run)


if __name__=='__main__':unittest.main()

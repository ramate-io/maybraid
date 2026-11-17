#!/usr/bin/env python3
"""Verify generation_trace_fixture's completed Tracy 0.14.1 message export."""
import argparse
import csv
import json
from pathlib import Path
import re


def verify(path, detail_limit=None, zones=None):
    lines=Path(path).read_text().splitlines()
    if not lines or lines[0]!='MessageName,total_ns':
        raise ValueError('expected Tracy 0.14.1 -m export, MessageName,total_ns')
    messages=[]
    # This exporter leaves message commas unquoted. The last comma is the
    # timestamp delimiter; csv.DictReader would silently lose message fields.
    for line in lines[1:]:
        message,timestamp=line.rsplit(',',1)
        int(timestamp)
        if 'hcsg.' in message:messages.append(message)
    fields=lambda text:{key:value.strip() for key,value in re.findall(r'(\w+) = (.*?)(?:, |$)',text)}
    summaries=[fields(text) for text in messages if 'hcsg.operation.complete' in text]
    sync=[row for row in summaries if row.get('origin')=='fixture.sync']
    second=[row for row in summaries if row.get('origin')=='fixture.second']
    hits=[row for row in summaries if row.get('origin')=='fixture.cache-hit']
    worker=[row for row in summaries if row.get('epoch_known')=='true']
    if len(sync)!=1 or any(sync[0].get(k)!=v for k,v in [('builds','2'),('published','2'),('cache_hits','1')]):
        raise ValueError(f'incomplete synchronous accounting: {sync}')
    if len(second)!=1 or second[0].get('cache_hits')!='1' or second[0].get('builds')!='0':
        raise ValueError('out-of-order context accounting missing')
    if len(hits)!=1000 or any(row.get('cache_hits')!='1' or row.get('builds')!='0' or row.get('published')!='0' for row in hits):
        raise ValueError(f'cache hit completion mismatch: {len(hits)}/1000')
    if len(worker)!=1 or worker[0].get('builds')!='0' or worker[0].get('cache_hits')!='1' or int(worker[0].get('queue_wait_ns','0'))<=0:
        raise ValueError('worker cache/queue accounting missing')
    quanta=[fields(text) for text in messages if 'hcsg.quantum.complete' in text]
    if len(quanta)!=1 or any(quanta[0].get(k)!=worker[0].get(k) for k in ['subscription','epoch','quantum']):
        raise ValueError('worker completion correlation mismatch')
    cancelled=[fields(text) for text in messages if 'hcsg.cancel' in text and 'reason = replaced' in text]
    enqueued=[fields(text) for text in messages if 'hcsg.enqueue' in text]
    if len(cancelled)!=1 or not any(row.get('subscription')==cancelled[0].get('subscription') and row.get('epoch')==cancelled[0].get('epoch') for row in enqueued):
        raise ValueError('replacement cancellation/enqueue correlation missing')
    if not any(row.get('subscription')==worker[0].get('subscription') and row.get('epoch')==worker[0].get('epoch') for row in enqueued):
        raise ValueError('worker enqueue correlation missing')
    if any(int(row.get('enqueue_age_ns','-1'))<0 for row in enqueued):
        raise ValueError('enqueue age missing')
    zone_result={}
    if zones is not None:
        with Path(zones).open() as source: zone_rows=list(csv.DictReader(source))
        if not zone_rows: raise ValueError('operation zones missing')
        names={row['name'] for row in zone_rows}
        for name in names:
            if not name.startswith('hcsg.operation{') or set(re.findall(r'(\w+)=',name))!={'origin','family'}:
                raise ValueError(f'operation zone contains unstable fields: {name}')
        hit_zones=[row for row in zone_rows if 'origin="fixture.cache-hit"' in row['name']]
        if len(hit_zones)!=1 or int(hit_zones[0]['counts'])!=1000:
            raise ValueError(f'cache-hit operation names did not collapse to one 1000-entry zone: {hit_zones}')
        zone_result=dict(stable_operation_zone_names=len(names),cache_hit_zone_entries=1000)
    if detail_limit is None:
        if any(row.get('details')!='0' for row in summaries):raise ValueError('unexpected default detail output')
    elif int(sync[0]['details'])!=detail_limit or int(sync[0]['suppressed'])<=0:
        raise ValueError('bounded detail suppression missing')
    return dict(diagnostic_messages=len(messages),operation_completions=len(summaries),cache_hit_completions=len(hits),verified_sync_counts=True,verified_worker_correlation=True,detail_limit=detail_limit,**zone_result)


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('messages',type=Path)
    parser.add_argument('--detail-limit',type=int)
    parser.add_argument('--zones',type=Path,help='Tracy -f hcsg.operation aggregate export')
    args=parser.parse_args()
    print(json.dumps(verify(args.messages,args.detail_limit,args.zones),indent=2))

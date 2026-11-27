#!/usr/bin/env python3
"""Offline numeric bake checks. numpy required, no source directories modified."""
from pathlib import Path
import argparse,json,struct,hashlib
import numpy as np
from surrealist_reference import EXPECTED_COMMIT
ROOT=Path(__file__).resolve().parents[2]
def sample(field,points,lo,hi):
 dims=np.array(field.shape[::-1]);q=(points-lo)/(hi-lo)*dims-.5;i=np.floor(q).astype(int);f=q-i;v=np.zeros(len(points));inside=np.all((points>=lo)&(points<=hi),axis=1)
 for z in [0,1]:
  for y in [0,1]:
   for x in [0,1]:
    j=np.clip(i+[x,y,z],0,dims-1);v+=field[j[:,2],j[:,1],j[:,0]]*(f[:,0]if x else 1-f[:,0])*(f[:,1]if y else 1-f[:,1])*(f[:,2]if z else 1-f[:,2])
 return np.where(inside,v,0)
def main():
 parser=argparse.ArgumentParser(description=__doc__)
 parser.add_argument('--assets-dir',type=Path,default=ROOT/'maybraid/assets/vfx/surrealist_dust')
 parser.add_argument('--readbacks',type=Path,required=True,help='Sequence reader output')
 parser.add_argument('--comparison-readbacks',type=Path,required=True,help='Comparison reader output with 00/03/06 samples')
 parser.add_argument('--report',type=Path,default=ROOT/'target/surrealist-export/numeric-validation.json')
 args=parser.parse_args();ASSETS=args.assets_dir.expanduser().resolve()
 for label,path in [('assets',ASSETS),('sequence readbacks',args.readbacks),('comparison readbacks',args.comparison_readbacks)]:
  if not path.is_dir():parser.error(f'{label} directory does not exist: {path}')
 m=json.loads((ASSETS/'surrealist.volume_flipbook.json').read_text());p=json.loads((ASSETS/'provenance.json').read_text());lo=np.array(m['bounds']['min']);hi=np.array(m['bounds']['max']);report={}
 assert p['reference_commit']==EXPECTED_COMMIT
 assert len(m['frames'])==48 and p['decoded_bytes']==100663296 and p['decoded_bytes']<=128*1024**2
 for i,name in enumerate(m['frames']):
  b=(ASSETS/name).read_bytes();assert hashlib.sha256(b).hexdigest()==p['sha256'][name];assert b[:12]==b'\xabKTX 20\xbb\r\n\x1a\n';assert struct.unpack_from('<I',b,12)[0]==76
  f=np.frombuffer(b[152:],dtype='<f2').reshape(128,64,128);assert np.isfinite(f).all()and(f>=0).all();assert np.max(f[[0,-1],:,:])==0 and np.max(f[:,:,[0,-1]])==0
  if i in [0,47]:assert not f.any()
 rng=np.random.default_rng(13)
 for frame,ref in [(2,0),(10,3),(38,6)]:
  source=args.readbacks/f'{frame:02}.rgba16';other=args.comparison_readbacks/f'{ref:02}.rgba16';assert source.read_bytes()==other.read_bytes()
  meta=json.loads((args.readbacks/f'{frame:02}.json').read_text());a=np.frombuffer(source.read_bytes(),dtype='<f2').reshape(160,96,192,4)[...,0]
  b=np.frombuffer((ASSETS/f'frame_{frame:02}.ktx2').read_bytes()[152:],dtype='<f2').reshape(128,64,128)
  pts=rng.uniform(meta['bounds_min'][:3],meta['bounds_max'][:3],(20000,3));va=sample(a,pts,np.array(meta['bounds_min'][:3]),np.array(meta['bounds_max'][:3]));vb=sample(b,pts,lo,hi);active=(va>.001)|(vb>.001);err=np.abs(va[active]-vb[active]);report[str(frame/10)]={'reference_readback_byte_identical':True,'active_probes':int(active.sum()),'mean_abs_density_error':float(err.mean()),'p95_abs_density_error':float(np.quantile(err,.95)),'max_abs_density_error':float(err.max())}
 args.report.parent.mkdir(parents=True,exist_ok=True);args.report.write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report,indent=2))
if __name__=='__main__':main()

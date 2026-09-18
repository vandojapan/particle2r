#!/usr/bin/env python3
"""Reproduce static evidence; never load or execute the supplied DLL/EXEs.
Usage: python reproduce_static.py SOURCE.zip OUTPUT_DIRECTORY
Requires Python 3 and GNU objdump on PATH. Fingerprint-specific registry location.
"""
import argparse, collections, hashlib, json, pathlib, re, struct, subprocess, tempfile, zipfile
EXPECTED='4a7b00013f819c83bc4f908d4b3f76c0cc469f5db795b6b0de127b9a74d51664'
def main():
 ap=argparse.ArgumentParser();ap.add_argument('zip');ap.add_argument('output');a=ap.parse_args()
 out=pathlib.Path(a.output);out.mkdir(parents=True,exist_ok=True)
 def write(n,v):
  (out/n).write_text(v if isinstance(v,str) else json.dumps(v,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
 inventory=[];raw={}
 with zipfile.ZipFile(a.zip) as z:
  if sum(i.file_size for i in z.infolist())>100*1024*1024:raise ValueError('Unexpected expanded size')
  for i in z.infolist():
   if i.is_dir():continue
   name=i.filename if i.flag_bits&2048 else i.filename.encode('cp437').decode('cp932')
   data=z.read(i);raw[name]=data
   inventory.append({'path':name,'size':len(data),'sha256':hashlib.sha256(data).hexdigest()})
 dllname=next(n for n in raw if n.endswith('/particle_set3.dll'));b=raw[dllname]
 if hashlib.sha256(b).hexdigest()!=EXPECTED:raise ValueError('Different DLL: registry RVA must be rediscovered')
 u16=lambda o:struct.unpack_from('<H',b,o)[0]
 u32=lambda o:struct.unpack_from('<I',b,o)[0]
 pe=u32(0x3c);opt=pe+24;base=u32(opt+28);secs=[]
 for i in range(u16(pe+6)):
  o=opt+u16(pe+20)+40*i
  secs.append({'name':b[o:o+8].rstrip(b'\0').decode(),'rva':u32(o+12),'virtual_size':u32(o+8),'raw_size':u32(o+16),'file_offset':u32(o+20)})
 def offset(va):
  rva=va-base
  for s in secs:
   if s['rva']<=rva<s['rva']+s['raw_size']:return s['file_offset']+rva-s['rva']
  raise ValueError(hex(va))
 def string(va):
  o=offset(va);e=b.find(b'\0',o,min(o+512,len(b)))
  if e<0:raise ValueError('unterminated')
  return b[o:e].decode('cp932')
 exports=[]
 for i in range(20):
  n,f=struct.unpack_from('<II',b,offset(base+0xa5000)+8*i)
  if n==0:break
  exports.append({'name':string(n),'rva':hex(f-base),'va':hex(f)})
 names=['lua_getfield','lua_pushnumber','lua_call','lua_rawgeti','lua_tonumber','lua_toboolean','lua_remove','lua_pushstring','lua_settop','lua_type','lua_createtable','lua_rawseti','lua_pushnil','lua_setfield','lua_touserdata','luaL_openlibs','lua_tolstring','lua_objlen','lua_pushboolean','lua_pushvalue','lua_isnumber','lua_isstring','luaL_loadfile','lua_pcall','luaL_register']
 tramp={base+0xa2610+8*i:n for i,n in enumerate(names)}
 with tempfile.TemporaryDirectory() as td:
  dll=pathlib.Path(td)/'particle_set3.dll';dll.write_bytes(b)
  headers=subprocess.check_output(['objdump','-x',str(dll)],text=True).replace(str(dll),'particle_set3.dll')
  dis=subprocess.check_output(['objdump','-d','-Mintel',str(dll)],text=True).replace(str(dll),'particle_set3.dll')
 write('pe_headers.txt',headers)
 annotated=[];refs=collections.defaultdict(list)
 for line in dis.splitlines():
  m=re.match(r'\s*([0-9a-f]+):',line)
  if not m:annotated.append(line);continue
  va=int(m[1],16);comments=[]
  for v in [int(h,16) for h in re.findall(r'0x([0-9a-f]+)',line)]:
   if v in tramp:comments.append(tramp[v])
   if base+0xa6000<=v<base+0xa7d28:
    try:s=string(v)
    except (UnicodeDecodeError,ValueError):continue
    if 1<=len(s)<=180 and all(c.isprintable() for c in s):
     comments.append(repr(s));refs[s].append({'rva':hex(va-base),'va':hex(va)})
  annotated.append(line+(' ; '+' | '.join(comments) if comments else ''))
 write('annotated_disassembly.txt','\n'.join(annotated)+'\n')
 write('string_xrefs.json',refs)
 write('lua_exports.json',exports)
 write('inventory.json',inventory)
 write('binary_summary.json',{'zip_sha256':hashlib.sha256(pathlib.Path(a.zip).read_bytes()).hexdigest(),'dll_sha256':EXPECTED,'dll_bytes':len(b),'machine':hex(u16(pe+4)),'optional_header_magic':hex(u16(opt)),'image_base':hex(base),'sections':secs,'lua_functions':exports,'file_count':len(inventory),'counts_by_suffix':dict(collections.Counter(pathlib.Path(i['path']).suffix for i in inventory))})
 text=raw[next(n for n in raw if n.endswith('/@particle_ri_ver3.anm'))].decode('cp932').replace('\r\n','\n')
 parts=list(re.finditer(r'^@([^\n]+)\n',text,re.M));params=[];catalog=['# 原版32項目の設定カタログ','', '抽出元：@particle_ri_ver3.anm。ラベル、範囲、既定値、配列順は原文のまま。Luaは実行していない。','']
 for j,m in enumerate(parts):
  block=text[m.end():parts[j+1].start() if j+1<len(parts) else len(text)]
  tracks=[];dialogs=[];checks=[]
  for l in block.splitlines():
   mt=re.match(r'--track(\d+):(.*)',l)
   if mt:
    fields=mt[2].split(',');tracks.append({'index':int(mt[1]),'label':fields[0],'min_raw':fields[1] if len(fields)>1 else None,'max_raw':fields[2] if len(fields)>2 else None,'default_raw':fields[3] if len(fields)>3 else None,'step_raw':fields[4] if len(fields)>4 else None,'raw':l})
   if l.startswith('--check'):checks.append(l)
   if l.startswith('--dialog:'):dialogs.append(l[len('--dialog:'):])
  assignments=re.findall(r'(?m)^([A-Za-z_][A-Za-z0-9_]*)\s*=\s*(\{[^\n]*\})',block)
  entry={'index':j+1,'name':m[1],'source_line':text[:m.start()].count('\n')+1,'tracks':tracks,'checks_raw':checks,'dialogs_raw':dialogs,'file_input':bool(re.search(r'^--file:',block,re.M)),'dll_calls':re.findall(r'particle_set3\.([A-Za-z_][A-Za-z0-9_]*)\(',block),'table_assignments_raw':[{'name':k,'value':v} for k,v in assignments]}
  params.append(entry)
  catalog+=['## '+str(j+1)+'. '+m[1],'',f"原スクリプト {entry['source_line']} 行目。",'', '```text']
  catalog += [t['raw'] for t in tracks]+checks+['--dialog:'+d for d in dialogs]
  catalog += ['```','','設定配列・DLL呼出：','', '```text']+[k+'='+v for k,v in assignments]+['particle_set3.'+n+'()' for n in entry['dll_calls']]+['```','']
 write('parameters.json',params);write('Parameter_Catalog.md','\n'.join(catalog))
 assert len(exports)==6 and len(params)==32 and len(inventory)==185
 print(json.dumps({'status':'ok','file_count':len(inventory),'effects':len(params),'lua_functions':len(exports),'output':str(out)},ensure_ascii=False))
if __name__=='__main__':main()

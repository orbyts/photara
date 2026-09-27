#!/usr/bin/env python3
"""New exact real-operation input after the frozen joined settled HEAD."""
import copy
import hashlib
import json
import struct
from pathlib import Path
HERE=Path(__file__).resolve().parent
SOURCE=HERE.parent/'operations/linked-operations.json'
def canonical(v):return json.dumps(v,sort_keys=True,separators=(',',':'),ensure_ascii=False,allow_nan=False).encode()
def sha(b):return hashlib.sha256(b).hexdigest()
def ref(v):
 b=canonical(v);return dict(kind='json',sha256=sha(b),byte_length=str(len(b)))
def build():
 original=json.loads(SOURCE.read_text());out=copy.deepcopy(original);pool=out['records']
 def save(name,v):
  b=canonical(v);pool[name]=dict(input=v,canonical=b.decode(),byte_length=len(b),sha256=sha(b));return ref(v)
 graph=copy.deepcopy(pool['graph-2']['input']);graph['graph']['revision']=3;graph['graph']['nodes'][0]['photara.graph-position']={'x':13,'y':14}
 authored=copy.deepcopy(pool['authored-2']['input']);authored['authored_revision']='4';authored['graphs'][0]['document']=save('graph-3',graph);save('authored-3',authored)
 operation='50000000-0000-4000-8000-000000000040'
 intent=copy.deepcopy(pool['intent-3']['input']);intent['operation_id']=operation;intent['expected']=original['expected_coordinates'][2]
 intent['command']['envelope'].update(command_id=operation,expected_revision=2);intent['command']['envelope']['command'].update(x=13,y=14);save('intent-4',intent)
 receipt=copy.deepcopy(pool['receipt-3']['input']);receipt.update(operation_id=operation,request_sha256=sha(canonical(intent)),acceptance_ordinal='4',journal_sequence='19',before=pool['receipt-3']['input']['after'],after=dict(revision='4',digest=ref(authored)['sha256']));save('receipt-4',receipt)
 frame=copy.deepcopy(pool['journal-frame-3']['input']);frame.update(sequence='19',operation_id=operation,request_sha256=receipt['request_sha256'],receipt_sha256=ref(receipt)['sha256']);save('journal-frame-4',frame)
 accepted=sha(canonical(dict(domain='photara.package.accepted-prefix-link.v1',previous_sha256=out['expected_accepted_prefixes'][-1],acceptance_ordinal='4',operation_id=operation,request_sha256=receipt['request_sha256'],receipt_sha256=ref(receipt)['sha256'])))
 journal=sha(canonical(dict(domain='photara.package.journal-prefix-link.v1',previous_sha256=out['expected_journal_prefixes'][-1],frame_sha256=sha(canonical(frame)))))
 out['expected_accepted_prefixes'].append(accepted);out['expected_journal_prefixes'].append(journal)
 coordinate=copy.deepcopy(original['expected_coordinates'][2]);coordinate['revision']='4';coordinate['authored_digest']=ref(authored)['sha256'];g=coordinate['graphs'][graph['graph_id']];g.update(revision='3',semantic_digest=sha(canonical(graph['graph'])),payload_digest=sha(canonical(graph['graph'])),envelope_digest=ref(graph)['sha256']);out['expected_coordinates'].append(coordinate)
 entries=[dict(operation_id=pool[f'receipt-{n}']['input']['operation_id'],request_sha256=pool[f'receipt-{n}']['input']['request_sha256'],acceptance_ordinal=str(n),receipt=ref(pool[f'receipt-{n}']['input'])) for n in range(1,5)]
 ids=copy.deepcopy(pool['id-index-3']['input']);ids.update(count='4',entries=sorted(entries,key=lambda e:e['operation_id']));save('id-index-4',ids)
 ordinals=copy.deepcopy(pool['ordinal-index-3']['input']);ordinals.update(count='4',entries=entries);save('ordinal-index-4',ordinals)
 index=copy.deepcopy(pool['operation-root-3']['input']);index.update(accepted=dict(through_ordinal='4',prefix_sha256=accepted),by_id=ref(ids),by_ordinal=ref(ordinals));save('operation-root-4',index)
 active=out['selected_roots']['active'];active.update(authored=ref(authored),operation_index=ref(index),accepted=index['accepted']);active['journal_inclusion'].update(through_sequence='19',prefix_sha256=journal,resulting_authored_revision='4',resulting_authored_sha256=ref(authored)['sha256'])
 remove={'graph-2','authored-2','id-index-3','ordinal-index-3','operation-root-3'};names=[n for n in out['selected_record_names'] if n not in remove]+['graph-3','authored-3','receipt-4','id-index-4','ordinal-index-4','operation-root-4']
 pack=bytearray();locations=[]
 for name in names:
  v=pool[name]['input'];body=canonical(v);raw=b'PS2PKD01'+bytes([1,0,0,0])+struct.pack('<I',len(body))+body
  locations.append(dict(object=ref(v),membership='semantic',physical=dict(allocation_id=out['allocation_id'],arena='data',offset=str(len(pack)),byte_length=str(len(raw)),record_sha256=sha(raw))))
  pack.extend(raw);pool[name].update(frame_hex=raw.hex(),frame_sha256=sha(raw),frame_length=len(raw))
 locator=copy.deepcopy(pool['locator']['input']);locator.update(count=str(len(locations)),entries=sorted(locations,key=lambda e:(e['object']['sha256'],int(e['object']['byte_length']))));save('locator',locator)
 out.update(pack_hex=pack.hex(),pack_sha256=sha(pack),selected_record_names=names,original_operation_corpus_sha256=sha(SOURCE.read_bytes()),status='unfrozen-route-fourth-core-operation')
 return out
if __name__=='__main__':
 out=build();(HERE/'operation-four.json').write_text(json.dumps(out,sort_keys=True,indent=2)+'\n');print(sha(canonical(out['records']['receipt-4']['input'])))

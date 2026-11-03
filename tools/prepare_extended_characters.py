from pathlib import Path
import ast,subprocess,concurrent.futures,csv,json
from PIL import Image
ROOT=Path(__file__).resolve().parents[1]/'assets'
parts={
'f_body':'body/bodies/female','f_head':'head/heads/human/female',
'f_face_neutral':'head/faces/female/neutral','f_face_happy':'head/faces/female/happy','f_face_serious':'head/faces/female/anger',
'f_pants':'legs/pants/thin','f_shoes':'feet/shoes/basic/thin',
'f_shirt_tee':'torso/clothes/shortsleeve/tshirt/female','f_shirt_long':'torso/clothes/longsleeve/longsleeve/female','f_shirt_scoop':'torso/clothes/shortsleeve/tshirt_scoop/female',
'f_shirt_blouse':'torso/clothes/blouse/female','f_shirt_tunic':'torso/clothes/tunic/female','f_shirt_dress':'torso/clothes/robe/female',
'shirt_vest':'torso/clothes/vest/male','shirt_formal':'torso/clothes/longsleeve/formal/male','shirt_cardigan':'torso/clothes/longsleeve/longsleeve2_cardigan/male',
'shorts':'legs/shorts/shorts/male','f_shorts':'legs/shorts/shorts/thin','skirt':'legs/skirts/plain/male','f_skirt':'legs/skirts/straight/thin','f_long_skirt':'legs/skirts/belle/thin',
'hair_bun':'hair/bangs_bun/adult','hair_braid':'hair/braid/adult/fg','hair_braid_back':'hair/braid/adult/bg','hair_curls':'hair/curly_long/adult','hair_pigtails':'hair/pigtails/adult',
'acc_tiara':'hat/formal/tiara/adult','acc_bandana':'hat/cloth/bandana/adult','f_acc_hijab':'hat/cloth/hijab/thin'}
base='https://raw.githubusercontent.com/LiberatedPixelCup/Universal-LPC-Spritesheet-Character-Generator/master/spritesheets/'
def fetch(path):
 file=ROOT/'lpc'/path;file.parent.mkdir(parents=True,exist_ok=True)
 if not file.exists():
  result=subprocess.run(['curl','-fsSL',base+path,'-o',str(file)],capture_output=True)
  if result.returncode:
   color='orange' if path.startswith('hair/') else 'light' if path.startswith(('body/','head/')) else 'white'
   alternate=path[:-4]+'/'+color+'.png'
   result=subprocess.run(['curl','-fsSL',base+alternate,'-o',str(file)],capture_output=True)
   if result.returncode:return None
   (file.with_suffix('.source.txt')).write_text(base+alternate)
  else:(file.with_suffix('.source.txt')).write_text(base+path)
 return file
files=[p+'/'+anim+'.png' for p in parts.values() for anim in ['walk','idle']]
with concurrent.futures.ThreadPoolExecutor(max_workers=6)as pool:list(pool.map(fetch,files))
for name,path in parts.items():
 walk=Image.open(ROOT/'lpc'/path/'walk.png').convert('RGBA');out=Image.new('RGBA',(576,512));out.alpha_composite(walk)
 idle=ROOT/'lpc'/path/'idle.png'
 if idle.exists():out.alpha_composite(Image.open(idle).convert('RGBA'),(0,256))
 else:
  for row in range(4):
   frame=walk.crop((0,row*64,64,(row+1)*64));out.alpha_composite(frame,(0,256+row*64));out.alpha_composite(frame,(64,256+row*64))
 out.save(ROOT/'characters'/(name+'.png'))
rows=list(csv.DictReader(open(ROOT/'LPC-CREDITS.csv')))
selected=json.loads((ROOT/'character-credits.json').read_text());known={r['downloaded_file'] for r in selected}
for file in files:
 if file in known or not (ROOT/'lpc'/file).exists():continue
 matches=[r for r in rows if r['filename']==file or r['filename'].replace('${head}','female')==file]
 if not matches:raise Exception('Missing source credit: '+file)
 for row in matches:selected.append(dict(row,downloaded_file=file))
(ROOT/'character-credits.json').write_text(json.dumps(selected,indent=2))
credits='Extended avatar layers: female body, matching head/expressions/clothing, skirts and shorts, additional hairstyles and accessories. Source: Universal LPC Spritesheet Character Generator. Changes: walk/idle atlas assembly and optional palette recoloring. Source license alternatives retained below; selected OGA-BY 3.0 when available, otherwise CC0 or CC-BY-SA 3.0. Combined/recolored output is CC-BY-SA 4.0.\n\n'
for r in selected:
 if r['downloaded_file']in files:credits+=r['downloaded_file']+'\nAuthors: '+r['authors'].strip()+'\nLicenses: '+r['licenses'].strip()+'\nSources: '+r['urls'].strip()+'\nNotes: '+r['notes'].strip()+'\n\n'
original=(ROOT/'ASSET_CREDITS.txt').read_text();marker='\nEXTENDED AVATAR LAYERS\n';original=original.split(marker)[0];(ROOT/'ASSET_CREDITS.txt').write_text(original+marker+credits)
print('Prepared',len(parts),'additional animated layers and preserved their credits.')

from PIL import Image
import pathlib,subprocess,csv,json,concurrent.futures
ROOT=pathlib.Path(__file__).resolve().parents[1]/'assets'
parts={
'body':'body/bodies/male', 'head':'head/heads/human/male',
'face_neutral':'head/faces/male/neutral','face_happy':'head/faces/male/happy','face_serious':'head/faces/male/anger',
'hair_bob':'hair/bob/adult','hair_afro':'hair/afro/adult','hair_long':'hair/long/adult','hair_messy':'hair/messy1/adult','hair_ponytail':'hair/ponytail2/adult/fg','hair_ponytail_back':'hair/ponytail2/adult/bg',
'pants':'legs/pants/male','shirt_tee':'torso/clothes/shortsleeve/tshirt/male','shirt_scoop':'torso/clothes/shortsleeve/tshirt_scoop/male','shirt_long':'torso/clothes/longsleeve/longsleeve/male','shoes':'feet/shoes/basic/male',
'acc_glasses':'facial/glasses/halfmoon/adult','acc_wizard':'hat/magic/wizard/base/adult','acc_tophat':'hat/formal/tophat/adult','acc_hijab':'hat/cloth/hijab/male'}
base='https://raw.githubusercontent.com/LiberatedPixelCup/Universal-LPC-Spritesheet-Character-Generator/master/spritesheets/'
def fetch(p):
 path=ROOT/'lpc'/p;path.parent.mkdir(parents=True,exist_ok=True)
 if not path.exists():
  r=subprocess.run(['curl','-fsSL',base+p,'-o',str(path)],capture_output=True)
  if r.returncode:return None
 return path
files=[p+'/'+a+'.png' for p in parts.values() for a in ['walk','idle']]
with concurrent.futures.ThreadPoolExecutor(max_workers=6) as pool:list(pool.map(fetch,files))
(ROOT/'characters').mkdir(exist_ok=True)
for name,p in parts.items():
 walk=Image.open(ROOT/'lpc'/p/'walk.png').convert('RGBA')
 idlepath=ROOT/'lpc'/p/'idle.png'
 out=Image.new('RGBA',(576,512));out.alpha_composite(walk,(0,0))
 if idlepath.exists():out.alpha_composite(Image.open(idlepath).convert('RGBA'),(0,256))
 else:
  for row in range(4):
   crop=walk.crop((0,row*64,64,(row+1)*64))
   out.alpha_composite(crop,(0,256+row*64));out.alpha_composite(crop,(64,256+row*64))
 out.save(ROOT/'characters'/(name+'.png'))
# Preserve exact asset-specific attributions; template face paths are expanded to selected male heads.
rows=list(csv.DictReader(open(ROOT/'LPC-CREDITS.csv')));selected=[]
for f in files:
 matches=[r for r in rows if r['filename']==f or r['filename'].replace('${head}','male')==f]
 if not matches: print('MISSING CREDIT',f)
 for r in matches:
  r=dict(r);r['downloaded_file']=f;selected.append(r)
(ROOT/'character-credits.json').write_text(json.dumps(selected,indent=2))
print('Prepared',len(parts),'layer sheets; credit records',len(selected))

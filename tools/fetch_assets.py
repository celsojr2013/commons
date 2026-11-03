import subprocess,pathlib,json,csv,concurrent.futures
ROOT=pathlib.Path(__file__).resolve().parents[1]/'assets';ROOT.mkdir(exist_ok=True)
BASE='https://raw.githubusercontent.com/LiberatedPixelCup/Universal-LPC-Spritesheet-Character-Generator/master/'
def fetch(url,path):
 p=ROOT/path;p.parent.mkdir(parents=True,exist_ok=True)
 subprocess.run(['curl','-fsSL',url,'-o',str(p)],check=True)
 return path
mirror='https://raw.githubusercontent.com/Two-Weeks-Team/openClawWorld/main/packages/client/public/assets/kenney/'
for a,b in [('interior/interior_tilemap.png','interior.png'),('tiles/tinytown_tilemap.png','town.png')]:fetch(mirror+a,b)
for path in ['body/bodies/male/walk.png','body/bodies/male/idle.png','head/heads/human/male/walk.png','head/heads/human/male/idle.png','head/faces/male/neutral/walk.png','head/faces/male/happy/walk.png','head/faces/male/anger/walk.png','hair/bob/adult/walk.png','hair/long/adult/walk.png','legs/pants/male/walk.png']:
 try:fetch(BASE+'spritesheets/'+path,'lpc/'+path)
 except Exception as e:print('FAIL',path)
for name in ['body/body_ulpc.json','hair/hair_ulpc.json','cloth/cloth_ulpc.json']:
 fetch(BASE+'palette_definitions/'+name,'palettes/'+name.split('/')[-1])
fetch(BASE+'CREDITS.csv','LPC-CREDITS.csv')

from PIL import Image,ImageDraw
import pathlib,json,random,math
ROOT=pathlib.Path(__file__).resolve().parents[1]/'assets'
interior=Image.open(ROOT/'interior.png').convert('RGBA');town=Image.open(ROOT/'town.png').convert('RGBA')
items=[]
def add(name,category,image,solid=False,source='Original Commons pixel art'):
 id=len(items);items.append({'id':id,'name':name,'category':category,'width':image.width,'height':image.height,'solid':solid,'source':source,'image':image});return id
def tile(x,y,w=1,h=1):
 im=Image.new('RGBA',(w*16,h*16))
 for j in range(h):
  for i in range(w):im.alpha_composite(interior.crop(((x+i)*17,(y+j)*17,(x+i)*17+16,(y+j)*17+16)),(i*16,j*16))
 return im
def pixel(w=16,h=16):return Image.new('RGBA',(w,h))
# Ground tiles are seamless, created for this app.
for name,base,line in [('Honey oak','#b48a64','#987153'),('Walnut','#7a594d','#60463f'),('Pale oak','#cfb794','#b39a7d'),('Midnight wood','#514559','#3f3649')]:
 im=Image.new('RGBA',(16,16),base);d=ImageDraw.Draw(im)
 for y in [0,8]:d.line((0,y,15,y),fill=line);d.line((4 if y==0 else 12,y,4 if y==0 else 12,y+7),fill=line)
 d.point((9,4),fill=line);d.line((1,12,6,12),fill=line);add(name,'Floors',im)
for name,base,line in [('Warm stone','#a79d91','#8d857c'),('Slate','#747686','#626575'),('Ivory tile','#dad3c2','#bbb5a9')]:
 im=Image.new('RGBA',(16,16),base);d=ImageDraw.Draw(im);d.rectangle((0,0,15,15),outline=line);d.line((1,1,14,1),fill='#e1dac8' if name!='Slate' else '#858797');add(name,'Floors',im)
for name,base,dot in [('Grass','#77a36d','#638b59'),('Autumn grass','#a8a373','#8f9061'),('Forest moss','#597e65','#416b54'),('Lavender carpet','#8a719d','#7c638f'),('Sage carpet','#7e9c8c','#6c8a7a'),('Sand','#d6b586','#c2a171'),('Pond','#6b9eaf','#8dbac6')]:
 im=Image.new('RGBA',(16,16),base);d=ImageDraw.Draw(im);rng=random.Random(33)
 for i in range(20):x=rng.randrange(16);y=rng.randrange(16);d.point((x,y),fill=dot)
 if name=='Pond':d.line((2,4,6,4),fill=dot);d.line((10,12,14,12),fill=dot)
 add(name,'Floors',im)
# Walls and windows.
for name,base,edge in [('Cream wall','#e0ceb1','#ac9988'),('Sage wall','#9bb3a5','#6e8c7a'),('Plum wall','#81708c','#5a4c65'),('Castle brick','#8894a3','#5c687c')]:
 im=Image.new('RGBA',(16,16),base);d=ImageDraw.Draw(im);d.rectangle((0,13,15,15),fill=edge);d.line((0,0,15,0),fill=edge)
 if name=='Castle brick':
  for y in [4,9]:d.line((0,y,15,y),fill=edge)
  for x,y in [(4,1),(12,5),(4,10)]:d.line((x,y,x,y+3),fill=edge)
 add(name,'Walls',im,True)
im=Image.new('RGBA',(16,16),'#e0ceb1');d=ImageDraw.Draw(im);d.rectangle((2,1,13,12),fill='#78675e');d.rectangle((3,2,12,11),fill='#94c1cb');d.line((8,2,8,11),fill='#dfd0b5');d.line((3,7,12,7),fill='#dfd0b5');d.line((2,13,13,13),fill='#ac9988');add('Sunlit window','Walls',im,True)
# Furniture combines exact tiles from Kenney's CC0 interior pack.
for name,x,y,w,h,solid in [('Long table',0,0,3,2,True),('Round table',6,0,2,2,True),('Oak chair',0,2,1,1,True),('Side chair',1,2,1,1,True),('Couch',9,6,3,2,True),('Armchair',8,6,1,2,True),('Bookshelf',19,17,1,1,True),('Cabinet',0,15,1,2,True),('Kitchen counter',0,12,3,1,True),('Stove',14,14,1,2,True),('Fridge',11,14,1,3,True),('Coffee table',4,4,2,1,True),('Potted fern',16,0,1,1,True),('Little plant',17,0,1,1,True),('Candle',20,0,1,1,False),('Candelabra',19,0,1,1,False),('Wall map',18,0,1,1,False),('Gold rug',19,15,1,2,False),('Sage rug',19,14,1,1,False)]:
 add(name,'Nature' if 'plant' in name.lower() or 'fern' in name.lower() else 'Furniture',tile(x,y,w,h),solid,'Kenney / Roguelike Indoors (CC0)')
# Original pixel couches with readable continuous seats.
for item in items:
 if item['name'] in ['Couch','Armchair']:
  width=48 if item['name']=='Couch' else 16
  im=pixel(width,32);d=ImageDraw.Draw(im);d.rectangle((1,8,width-2,26),fill='#465c5a');d.rectangle((2,5,width-3,22),fill='#6f9380');d.rectangle((4,7,width-5,17),fill='#8aad91');d.rectangle((3,18,width-4,24),fill='#7b9f88');d.rectangle((1,15,4,26),fill='#567668');d.rectangle((width-5,15,width-2,26),fill='#567668');d.rectangle((3,27,5,29),fill='#514153');d.rectangle((width-6,27,width-4,29),fill='#514153')
  if width>16:
   for x in [16,31]:d.line((x,8,x,24),fill='#608372')
  item['image']=im;item['source']='Original Commons pixel art'
# An office desk adds an original pixel monitor over the sourced wooden table.
im=tile(0,0,3,2);d=ImageDraw.Draw(im);d.rectangle((14,2,34,14),fill='#383b50');d.rectangle((16,4,32,11),fill='#78b9b5');d.line((17,5,24,5),fill='#bce1cf');d.rectangle((23,15,25,17),fill='#383b50');d.rectangle((18,18,31,21),fill='#d3d5c9');d.rectangle((38,16,41,19),fill='#f2e4bb');add('Workstation','Furniture',im,True,'Kenney CC0 table + original Commons monitor')
for name,x,y,w,h in [('Pine tree',4,0,1,1),('Autumn tree',3,0,1,1),('Leafy tree',5,0,1,1),('Flowers',2,0,1,1),('Mushrooms',5,2,1,1),('Wood fence',8,3,3,1)]:
 im=town.crop((x*16,y*16,(x+w)*16,(y+h)*16))
 if 'tree' in name:im=im.resize((32,32),Image.Resampling.NEAREST)
 add(name,'Nature',im,name not in ['Flowers','Mushrooms'],'Kenney / Tiny Town (CC0)')
# Friendly Halloween decorations, drawn for Commons. No gore.
im=pixel();d=ImageDraw.Draw(im);d.rectangle((7,1,9,3),fill='#566d43');d.rectangle((3,5,12,13),fill='#d87937');d.rectangle((1,7,14,11),fill='#d87937');d.line((4,5,4,13),fill='#ef9b44');d.line((11,5,11,13),fill='#aa522c');d.rectangle((4,7,5,8),fill='#ffe294');d.rectangle((10,7,11,8),fill='#ffe294');d.line((5,11,10,11),fill='#ffe294');add('Jack-o-lantern','Halloween',im,True)
im=pixel(16,24);d=ImageDraw.Draw(im);d.rectangle((4,4,11,19),fill='#e4e6df');d.rectangle((2,7,13,19),fill='#e4e6df');d.rectangle((5,2,10,5),fill='#e4e6df');d.rectangle((4,20,6,21),fill='#e4e6df');d.rectangle((10,20,12,21),fill='#e4e6df');d.rectangle((5,9,6,11),fill='#596072');d.rectangle((10,9,11,11),fill='#596072');d.rectangle((8,14,9,16),fill='#8990a1');add('Friendly ghost','Halloween',im)
im=pixel(32,16);d=ImageDraw.Draw(im);d.line((1,2,30,2),fill='#4d405f');d.polygon([(2,2),(10,2),(6,10)],fill='#c38049');d.polygon([(12,2),(20,2),(16,10)],fill='#8b709e');d.polygon([(22,2),(30,2),(26,10)],fill='#c38049');add('Spooky bunting','Halloween',im)
im=pixel();d=ImageDraw.Draw(im)
for v in [3,7,11,15]:d.line((0,v,v,0),fill='#dcd8d2')
for x,y in [(0,15),(8,8),(15,0)]:d.line((0,0,x,y),fill='#dcd8d2')
add('Cobweb','Halloween',im)
im=pixel();d=ImageDraw.Draw(im);d.rectangle((4,9,11,14),fill='#61556f');d.rectangle((5,6,10,9),fill='#61556f');d.rectangle((6,3,9,6),fill='#61556f');d.rectangle((3,13,12,14),fill='#91809f');add('Witch hat decor','Halloween',im)
im=pixel();d=ImageDraw.Draw(im);d.rectangle((4,4,11,14),fill='#9ba1a1');d.rectangle((5,2,10,4),fill='#9ba1a1');d.line((8,5,8,11),fill='#626b75');d.line((6,7,10,7),fill='#626b75');add('Little gravestone','Halloween',im,True)
# Atlas packs each asset into a 64px slot; metadata carries exact bounds.
atlas=Image.new('RGBA',(512,math.ceil(len(items)/8)*64))
for item in items:
 i=item['id'];x=(i%8)*64;y=(i//8)*64;atlas.alpha_composite(item.pop('image'),(x,y));item['x']=x;item['y']=y
atlas.save(ROOT/'environment.png');(ROOT/'environment.json').write_text(json.dumps(items,indent=2));print('Prepared',len(items),'environment brushes')
